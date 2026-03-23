use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::projections::{Mutation, apply};
use crate::modules::time_entries::use_cases::list_time_entries::projection::{
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
                    state.rows.insert(row.time_entry_id.clone(), row);
                }
                Mutation::SetStartedAt {
                    time_entry_id,
                    started_at,
                    updated_at,
                    updated_by,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&time_entry_id) {
                        row.started_at = Some(started_at);
                        row.updated_at = updated_at;
                        row.updated_by = updated_by;
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetEndedAt {
                    time_entry_id,
                    ended_at,
                    updated_at,
                    updated_by,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&time_entry_id) {
                        row.ended_at = Some(ended_at);
                        row.updated_at = updated_at;
                        row.updated_by = updated_by;
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetRegistered {
                    time_entry_id,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&time_entry_id) {
                        row.status =
                            crate::modules::time_entries::use_cases::list_time_entries::projection::TimeEntryStatus::Registered;
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetDeleted {
                    time_entry_id,
                    deleted_at,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&time_entry_id) {
                        row.deleted_at = Some(deleted_at);
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetTags {
                    time_entry_id,
                    tag_ids,
                    updated_at,
                    updated_by,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&time_entry_id) {
                        row.tag_ids = tag_ids;
                        row.updated_at = updated_at;
                        row.updated_by = updated_by;
                        row.last_event_id = Some(last_event_id);
                    }
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
    use crate::modules::time_entries::core::events::v1::time_entry_deleted::TimeEntryDeletedV1;
    use crate::modules::time_entries::core::events::v1::time_entry_end_set::TimeEntryEndSetV1;
    use crate::modules::time_entries::core::events::v1::time_entry_initiated::TimeEntryInitiatedV1;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::modules::time_entries::core::events::v1::time_entry_start_set::TimeEntryStartSetV1;
    use crate::modules::time_entries::core::events::v1::time_entry_tags_set::TimeEntryTagsSetV1;
    use crate::modules::time_entries::use_cases::set_ended_at::handler::SetEndedAtHandler;
    use crate::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
    use crate::shared::infrastructure::event_store::EventStore;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use crate::tests::fixtures::commands::set_ended_at::SetEndedAtBuilder;
    use crate::tests::fixtures::commands::set_started_at::SetStartedAtBuilder;
    use rstest::rstest;

    async fn initiate_and_register(
        event_store: InMemoryEventStore<TimeEntryEvent>,
        time_entry_id: &str,
        stream_id: &str,
    ) {
        let outbox = InMemoryDomainOutbox::new();
        SetStartedAtHandler::new("t", event_store.clone(), outbox.clone())
            .handle(
                stream_id,
                SetStartedAtBuilder::new()
                    .time_entry_id(time_entry_id.to_string())
                    .build(),
            )
            .await
            .unwrap();
        SetEndedAtHandler::new("t", event_store, outbox)
            .handle(
                stream_id,
                SetEndedAtBuilder::new()
                    .time_entry_id(time_entry_id.to_string())
                    .build(),
            )
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_rebuild_and_apply_on_schema_mismatch() {
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        initiate_and_register(event_store.clone(), "te-abc", "TimeEntry-abc").await;

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
        initiate_and_register(event_store.clone(), "te-abc", "TimeEntry-abc").await;

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
    async fn it_should_apply_initiate_event_from_channel_and_emit_event_applied() {
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
        SetStartedAtHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-1",
                SetStartedAtBuilder::new()
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
    async fn it_should_apply_all_mutation_types() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(64);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(64);
        let projector = ListTimeEntriesProjector::new(
            "p",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        // Append events covering all mutation types
        let events = vec![
            TimeEntryEvent::TimeEntryInitiatedV1(TimeEntryInitiatedV1 {
                time_entry_id: "te-mut".to_string(),
                user_id: "user-0001".to_string(),
                created_at: 1_000,
                created_by: "user-0001".to_string(),
            }),
            TimeEntryEvent::TimeEntryStartSetV1(TimeEntryStartSetV1 {
                time_entry_id: "te-mut".to_string(),
                started_at: 500,
                updated_at: 1_000,
                updated_by: "user-0001".to_string(),
            }),
            TimeEntryEvent::TimeEntryEndSetV1(TimeEntryEndSetV1 {
                time_entry_id: "te-mut".to_string(),
                ended_at: 800,
                updated_at: 1_000,
                updated_by: "user-0001".to_string(),
            }),
            TimeEntryEvent::TimeEntryRegisteredV1(TimeEntryRegisteredV1 {
                time_entry_id: "te-mut".to_string(),
                occurred_at: 1_000,
            }),
            TimeEntryEvent::TimeEntryTagsSetV1(TimeEntryTagsSetV1 {
                time_entry_id: "te-mut".to_string(),
                tag_ids: vec!["tag-1".to_string()],
                updated_at: 1_500,
                updated_by: "user-0001".to_string(),
            }),
            TimeEntryEvent::TimeEntryDeletedV1(TimeEntryDeletedV1 {
                time_entry_id: "te-mut".to_string(),
                deleted_at: 2_000,
                deleted_by: "user-0001".to_string(),
            }),
        ];
        event_store
            .append("TimeEntry-te-mut", 0, &events)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let state = projection_store.state().await.unwrap().unwrap();
        let row = state.rows.get("te-mut").expect("row should exist");
        use crate::modules::time_entries::use_cases::list_time_entries::projection::TimeEntryStatus;
        assert_eq!(row.started_at, Some(500));
        assert_eq!(row.ended_at, Some(800));
        assert_eq!(row.status, TimeEntryStatus::Registered);
        assert_eq!(row.tag_ids, vec!["tag-1".to_string()]);
        assert_eq!(row.deleted_at, Some(2_000));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_silently_skip_mutations_when_row_not_found() {
        let (tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(64);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(64);
        let projector = ListTimeEntriesProjector::new(
            "p",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        // Send SetStartedAt, SetEndedAt, SetRegistered, SetTags, and SetDeleted without a
        // preceding Initiated event — these should all be silently skipped (row not found)
        let events = vec![
            TimeEntryEvent::TimeEntryStartSetV1(TimeEntryStartSetV1 {
                time_entry_id: "te-orphan".to_string(),
                started_at: 1_000,
                updated_at: 2_000,
                updated_by: "u1".to_string(),
            }),
            TimeEntryEvent::TimeEntryEndSetV1(TimeEntryEndSetV1 {
                time_entry_id: "te-orphan".to_string(),
                ended_at: 3_000,
                updated_at: 2_000,
                updated_by: "u1".to_string(),
            }),
            TimeEntryEvent::TimeEntryRegisteredV1(TimeEntryRegisteredV1 {
                time_entry_id: "te-orphan".to_string(),
                occurred_at: 2_000,
            }),
            TimeEntryEvent::TimeEntryTagsSetV1(TimeEntryTagsSetV1 {
                time_entry_id: "te-orphan".to_string(),
                tag_ids: vec!["tag-1".to_string()],
                updated_at: 2_000,
                updated_by: "u1".to_string(),
            }),
            TimeEntryEvent::TimeEntryDeletedV1(TimeEntryDeletedV1 {
                time_entry_id: "te-orphan".to_string(),
                deleted_at: 4_000,
                deleted_by: "u1".to_string(),
            }),
        ];
        event_store
            .append("TimeEntry-te-orphan", 0, &events)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // No row should have been created since Initiated was never sent
        let state = projection_store.state().await.unwrap().unwrap();
        assert!(state.rows.is_empty());
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
        SetStartedAtHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-skip",
                SetStartedAtBuilder::new()
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
        SetStartedAtHandler::new("t", event_store, outbox)
            .handle(
                "TimeEntry-fail",
                SetStartedAtBuilder::new()
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

        initiate_and_register(event_store.clone(), "te-lag1", "TimeEntry-lag1").await;
        initiate_and_register(event_store.clone(), "te-lag2", "TimeEntry-lag2").await;

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
        initiate_and_register(event_store.clone(), "te-abc", "TimeEntry-abc").await;

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
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        initiate_and_register(event_store.clone(), "te-abc", "TimeEntry-abc").await;

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
