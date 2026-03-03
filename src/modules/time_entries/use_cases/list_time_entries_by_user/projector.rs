use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::projections::{Mutation, apply};
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::{
    ListTimeEntriesState, SCHEMA_VERSION,
};
use crate::shared::infrastructure::event_store::StoredEvent;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::projection_store::ProjectionStore;
use std::sync::Arc;
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
    pub store: Arc<TStore>,
    pub event_store: Arc<InMemoryEventStore<TimeEntryEvent>>,
    pub technical_tx: broadcast::Sender<ProjectionTechnicalEvent>,
}

impl<TStore> ListTimeEntriesProjector<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    pub fn new(
        name: impl Into<String>,
        store: Arc<TStore>,
        event_store: Arc<InMemoryEventStore<TimeEntryEvent>>,
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
        if stored_schema != Some(SCHEMA_VERSION) {
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
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use rstest::rstest;

    async fn make_event_store_with_one_event() -> Arc<InMemoryEventStore<TimeEntryEvent>> {
        let (tx, _) = broadcast::channel(16);
        let store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx));
        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let handler = RegisterTimeEntryHandler::new("t", store.clone(), outbox);
        handler
            .handle("TimeEntry-abc", RegisterTimeEntryBuilder::new().build())
            .await
            .unwrap();
        store
    }

    fn make_projector(
        store: Arc<InMemoryProjectionStore<ListTimeEntriesState>>,
        event_store: Arc<InMemoryEventStore<TimeEntryEvent>>,
    ) -> (
        ListTimeEntriesProjector<InMemoryProjectionStore<ListTimeEntriesState>>,
        broadcast::Receiver<ProjectionTechnicalEvent>,
    ) {
        let (tech_tx, tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new("test-proj", store, event_store, tech_tx);
        (projector, tech_rx)
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_apply_events_from_channel_when_schema_version_matches() {
        let (tx, _rx) = broadcast::channel(16);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(
            tx.clone(),
        ));
        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (projector, _tech_rx) = make_projector(projection_store.clone(), event_store.clone());
        let receiver = tx.subscribe();

        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let handler = RegisterTimeEntryHandler::new("topic", event_store.clone(), outbox);

        tokio::spawn(projector.run(receiver));

        handler
            .handle(
                "TimeEntry-1",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-1".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let cp = projection_store.checkpoint().await.unwrap();
        assert_eq!(cp, 1);
        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_rebuild_when_schema_version_mismatches() {
        let event_store = make_event_store_with_one_event().await;
        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());

        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let receiver = tx.subscribe();
        drop(tx);

        let (projector, mut tech_rx) =
            make_projector(projection_store.clone(), event_store.clone());
        projector.run(receiver).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 1);
        assert_eq!(
            projection_store.schema_version().await.unwrap(),
            Some(SCHEMA_VERSION)
        );

        let mut got_started = false;
        let mut got_completed = false;
        while let Ok(ev) = tech_rx.try_recv() {
            match ev {
                ProjectionTechnicalEvent::RebuildStarted { .. } => got_started = true,
                ProjectionTechnicalEvent::RebuildCompleted {
                    events_replayed, ..
                } => {
                    got_completed = true;
                    assert_eq!(events_replayed, 1);
                }
                _ => {}
            }
        }
        assert!(got_started);
        assert!(got_completed);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_emit_event_applied_technical_event() {
        let (tx, _rx) = broadcast::channel(16);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(
            tx.clone(),
        ));
        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new(
            "test-proj",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();

        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let handler = RegisterTimeEntryHandler::new("t", event_store.clone(), outbox);

        tokio::spawn(projector.run(receiver));
        handler
            .handle(
                "TimeEntry-2",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-2".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let ev = tech_rx.try_recv().unwrap();
        assert!(matches!(
            ev,
            ProjectionTechnicalEvent::EventApplied { checkpoint: 1, .. }
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_skip_events_already_covered_by_checkpoint() {
        let (tx, _) = broadcast::channel(16);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(
            tx.clone(),
        ));

        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let handler = RegisterTimeEntryHandler::new("t", event_store.clone(), outbox);
        handler
            .handle(
                "TimeEntry-x",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-x".to_string())
                    .build(),
            )
            .await
            .unwrap();

        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());

        let receiver = tx.subscribe();

        let (projector, _) = make_projector(projection_store.clone(), event_store.clone());

        tokio::spawn(projector.run(receiver));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 1);
        let cp = projection_store.checkpoint().await.unwrap();
        assert_eq!(cp, 1);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_trigger_rebuild_on_lagged_receiver() {
        // Event store uses its own sender (moved in, not cloned) so it doesn't
        // keep the projector's receiver channel alive.
        let (store_tx, _) = broadcast::channel(16);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(
            store_tx,
        ));

        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let outbox = Arc::new(InMemoryDomainOutbox::new());
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

        // Separate small-capacity channel for the projector's receiver.
        // Send 2 messages to a capacity-1 channel so the receiver lags,
        // then drop the sender so the channel closes after the rebuild.
        let (lag_tx, receiver) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);
        let dummy = event_store.load_all_from(0).await.unwrap().remove(0);
        lag_tx.send(dummy.clone()).unwrap();
        lag_tx.send(dummy).unwrap();
        drop(lag_tx);

        let (tech_tx, mut tech_rx) = broadcast::channel(32);
        let projector = ListTimeEntriesProjector::new(
            "lag-proj",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );

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
    async fn it_should_exit_run_loop_when_channel_is_closed() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx));
        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(4);
        let projector = ListTimeEntriesProjector::new("p", projection_store, event_store, tech_tx);
        let (closed_tx, closed_rx) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);
        drop(closed_tx);
        projector.run(closed_rx).await;
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_exit_and_emit_rebuild_failed_when_store_is_offline_on_startup() {
        let event_store = make_event_store_with_one_event().await;
        let mut projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store.toggle_offline();
        let projection_store = Arc::new(projection_store);

        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(16);
        let receiver = tx.subscribe();
        drop(tx);

        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector =
            ListTimeEntriesProjector::new("offline-proj", projection_store, event_store, tech_tx);

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
    async fn it_should_exit_and_emit_rebuild_failed_when_store_goes_offline_on_lag() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(
            tx.clone(),
        ));

        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let receiver = tx.subscribe();

        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let handler = RegisterTimeEntryHandler::new("t", event_store.clone(), outbox);
        handler
            .handle(
                "TimeEntry-of1",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-of1".to_string())
                    .build(),
            )
            .await
            .unwrap();
        handler
            .handle(
                "TimeEntry-of2",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-of2".to_string())
                    .build(),
            )
            .await
            .unwrap();

        let mut offline_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        offline_store.toggle_offline();
        let offline_store = Arc::new(offline_store);

        let (tech_tx, mut tech_rx) = broadcast::channel(32);
        let projector = ListTimeEntriesProjector::new(
            "lag-offline-proj",
            offline_store,
            event_store.clone(),
            tech_tx,
        );

        drop(tx);
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
    async fn it_should_continue_when_apply_stored_event_fails() {
        let (tx, _rx) = broadcast::channel(16);
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new_with_sender(
            tx.clone(),
        ));
        let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _tech_rx) = broadcast::channel(16);
        let projector = ListTimeEntriesProjector::new(
            "fail-apply",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();

        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let handler = RegisterTimeEntryHandler::new("t", event_store.clone(), outbox);

        tokio::spawn(projector.run(receiver));

        handler
            .handle(
                "TimeEntry-fa",
                RegisterTimeEntryBuilder::new()
                    .time_entry_id("te-fa".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert_eq!(projection_store.checkpoint().await.unwrap(), 1);
    }
}
