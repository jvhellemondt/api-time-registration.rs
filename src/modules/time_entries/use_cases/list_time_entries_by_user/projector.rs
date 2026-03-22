use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::projections::{Mutation, apply};
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::{
    ListTimeEntriesState, SCHEMA_VERSION,
};
use crate::shared::infrastructure::event_store::StoredEvent;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::projection_store::ProjectionStore;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum ProjectionTechnicalEvent {
    EventApplied {
        projection_name: String,
        checkpoint: u64,
        duration_ms: u64,
    },
    RebuildStarted {
        projection_name: String,
        schema_version: u32,
        timestamp: i64,
    },
    RebuildCompleted {
        projection_name: String,
        events_replayed: u64,
        duration_ms: u64,
        timestamp: i64,
    },
    RebuildFailed {
        projection_name: String,
        reason: String,
        timestamp: i64,
    },
}

pub struct ListTimeEntriesProjector<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    pub name: String,
    pub store: TStore,
    pub event_store: InMemoryEventStore<TimeEntryEvent>,
    pub technical_tx: broadcast::Sender<ProjectionTechnicalEvent>,
}

impl<TStore> ListTimeEntriesProjector<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    pub fn new(
        name: impl Into<String>,
        store: TStore,
        event_store: InMemoryEventStore<TimeEntryEvent>,
        technical_tx: broadcast::Sender<ProjectionTechnicalEvent>,
    ) -> Self {
        Self {
            name: name.into(),
            store,
            event_store,
            technical_tx,
        }
    }

    pub async fn run(self, mut receiver: broadcast::Receiver<StoredEvent<TimeEntryEvent>>) {
        let stored_schema = self.store.schema_version().await.unwrap_or(None);
        if stored_schema != Some(SCHEMA_VERSION)
            && let Err(reason) = self.rebuild().await
        {
            let _ = self
                .technical_tx
                .send(ProjectionTechnicalEvent::RebuildFailed {
                    projection_name: self.name.clone(),
                    reason: reason.to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                });
            return;
        }

        loop {
            match receiver.recv().await {
                Ok(stored_event) => {
                    let checkpoint = self.store.checkpoint().await.unwrap_or(0);
                    if stored_event.global_position < checkpoint {
                        continue;
                    }
                    let start = std::time::Instant::now();
                    if self.apply_stored_event(&stored_event).await.is_err() {
                        continue;
                    }
                    let _ = self
                        .technical_tx
                        .send(ProjectionTechnicalEvent::EventApplied {
                            projection_name: self.name.clone(),
                            checkpoint: stored_event.global_position + 1,
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if let Err(reason) = self.rebuild().await {
                        let _ = self
                            .technical_tx
                            .send(ProjectionTechnicalEvent::RebuildFailed {
                                projection_name: self.name.clone(),
                                reason: reason.to_string(),
                                timestamp: chrono::Utc::now().timestamp_millis(),
                            });
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }

    async fn rebuild(&self) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        let _ = self
            .technical_tx
            .send(ProjectionTechnicalEvent::RebuildStarted {
                projection_name: self.name.clone(),
                schema_version: SCHEMA_VERSION,
                timestamp: chrono::Utc::now().timestamp_millis(),
            });
        self.store.clear().await?;
        let all_events = self.event_store.load_all_from(0).await?;
        let events_replayed = all_events.len() as u64;
        for stored_event in all_events {
            self.apply_stored_event(&stored_event).await?;
        }
        self.store.save_schema_version(SCHEMA_VERSION).await?;
        let _ = self
            .technical_tx
            .send(ProjectionTechnicalEvent::RebuildCompleted {
                projection_name: self.name.clone(),
                events_replayed,
                duration_ms: start.elapsed().as_millis() as u64,
                timestamp: chrono::Utc::now().timestamp_millis(),
            });
        Ok(())
    }

    async fn apply_stored_event(
        &self,
        stored_event: &StoredEvent<TimeEntryEvent>,
    ) -> anyhow::Result<()> {
        let mut state = self.store.state().await?.unwrap_or_default();
        for mutation in apply(
            &stored_event.stream_id,
            stored_event.stream_version,
            &stored_event.event,
        ) {
            match mutation {
                Mutation::Upsert(row) => {
                    state
                        .rows
                        .insert((row.user_id.clone(), row.time_entry_id.clone()), row);
                }
            }
        }
        self.store
            .save(state, stored_event.global_position + 1)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod list_time_entries_projector_tests {
    use super::*;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use rstest::rstest;

    async fn register_one_entry(event_store: InMemoryEventStore<TimeEntryEvent>) {
        let outbox = InMemoryDomainOutbox::new();
        RegisterTimeEntryHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-abc",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-abc".to_string())
                    .build(),
            )
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_rebuild_and_apply_on_schema_mismatch() {
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        register_one_entry(event_store.clone()).await;

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        // Schema not set → mismatch → rebuild
        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        // Use a pre-closed channel so the projector exits after rebuild
        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        drop(closed_tx);
        let projector =
            ListTimeEntriesProjector::new("p", projection_store.clone(), event_store, tech_tx);
        projector.run(receiver).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 1);

        let mut got_rebuild = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::RebuildCompleted { .. }) {
                got_rebuild = true;
            }
        }
        assert!(got_rebuild);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_emit_rebuild_failed_and_exit_when_store_offline_at_startup() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());
        register_one_entry(event_store.clone()).await;

        let mut projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store.toggle_offline();

        let receiver = tx.subscribe();
        drop(tx);

        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new("p", projection_store, event_store, tech_tx);
        projector.run(receiver).await;

        let mut got_failed = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::RebuildFailed { .. }) {
                got_failed = true;
            }
        }
        assert!(got_failed);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_apply_event_from_channel_and_emit_event_applied() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new(
            "p",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        let outbox = InMemoryDomainOutbox::new();
        RegisterTimeEntryHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-1",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-1".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 1);

        let mut got_applied = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::EventApplied { .. }) {
                got_applied = true;
            }
        }
        assert!(got_applied);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_skip_event_already_in_checkpoint() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();
        // Set checkpoint ahead of any events
        projection_store
            .save(ListTimeEntriesState::default(), 999)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new(
            "p",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        let outbox = InMemoryDomainOutbox::new();
        RegisterTimeEntryHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-skip",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-skip".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // State rows should still be empty since the event was skipped
        let state = projection_store.state().await.unwrap().unwrap();
        assert!(state.rows.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_continue_when_apply_stored_event_fails() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let mut projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new(
            "p",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Toggle offline so apply_stored_event fails
        projection_store.toggle_offline();

        let outbox = InMemoryDomainOutbox::new();
        RegisterTimeEntryHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-fail",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-fail".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        // Projector continued (didn't crash); checkpoint is still 0 from before (store was offline for save)
        projection_store.toggle_offline();
        let cp = projection_store.checkpoint().await.unwrap();
        // Checkpoint should not have advanced (apply failed)
        assert_eq!(cp, 0);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_trigger_rebuild_on_lagged_receiver() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (lag_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);

        let outbox = InMemoryDomainOutbox::new();
        let handler = RegisterTimeEntryHandler::new("t", event_store.clone(), outbox);
        handler
            .handle(
                "TimeEntry-lag1",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-lag1".to_string())
                    .build(),
            )
            .await
            .unwrap();
        handler
            .handle(
                "TimeEntry-lag2",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-lag2".to_string())
                    .build(),
            )
            .await
            .unwrap();

        let dummy = event_store.load_all_from(0).await.unwrap().remove(0);
        lag_tx.send(dummy.clone()).unwrap();
        lag_tx.send(dummy).unwrap();
        drop(lag_tx);

        let (tech_tx, mut tech_rx) = broadcast::channel(32);
        let projector =
            ListTimeEntriesProjector::new("p", projection_store.clone(), event_store, tech_tx);
        projector.run(receiver).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 2);

        let mut got_rebuild = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::RebuildCompleted { .. }) {
                got_rebuild = true;
            }
        }
        assert!(got_rebuild);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_emit_rebuild_failed_and_exit_on_lagged_offline_store() {
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        register_one_entry(event_store.clone()).await;

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        // Save schema version so the initial check passes (no startup rebuild)
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        // Cause lag: capacity-1 channel with 2 events pre-loaded
        let (lag_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);
        let dummy = event_store.load_all_from(0).await.unwrap().remove(0);
        lag_tx.send(dummy.clone()).unwrap();
        lag_tx.send(dummy).unwrap();
        drop(lag_tx);

        // Toggle event_store offline so the rebuild triggered by lag fails
        event_store.toggle_offline();

        let (tech_tx, mut tech_rx) = broadcast::channel(32);
        let projector = ListTimeEntriesProjector::new("p", projection_store, event_store, tech_tx);
        projector.run(receiver).await;

        let mut got_failed = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::RebuildFailed { .. }) {
                got_failed = true;
            }
        }
        assert!(got_failed);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_rebuild_when_apply_stored_event_errors() {
        // Covers: line 128 (apply_stored_event?.await? error path) and
        //         line 162 (store.save().await? error path inside apply_stored_event)
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        register_one_entry(event_store.clone()).await;

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        // No schema_version set → mismatch → rebuild will be triggered.
        // Set fail_next_save so save() inside apply_stored_event fails.
        projection_store.set_fail_next_save();

        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        drop(closed_tx);
        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new("p", projection_store, event_store, tech_tx);
        projector.run(receiver).await;

        let mut got_failed = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::RebuildFailed { .. }) {
                got_failed = true;
            }
        }
        assert!(got_failed);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_rebuild_when_save_schema_version_errors() {
        // Covers: line 130 (store.save_schema_version().await? error path in rebuild)
        // Use an empty event store so the for loop does not run, then save_schema_version fails.
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        // No schema_version set → mismatch → rebuild triggered.
        projection_store.set_fail_next_save_schema_version();

        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        drop(closed_tx);
        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new("p", projection_store, event_store, tech_tx);
        projector.run(receiver).await;

        let mut got_failed = false;
        while let Ok(ev) = tech_rx.try_recv() {
            if matches!(ev, ProjectionTechnicalEvent::RebuildFailed { .. }) {
                got_failed = true;
            }
        }
        assert!(got_failed);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_exit_when_channel_closed() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);
        drop(closed_tx);

        let (tech_tx, _) = broadcast::channel(4);
        let projector = ListTimeEntriesProjector::new("p", projection_store, event_store, tech_tx);
        projector.run(receiver).await;
        // If we reach here, the projector exited cleanly on Closed
    }
}
