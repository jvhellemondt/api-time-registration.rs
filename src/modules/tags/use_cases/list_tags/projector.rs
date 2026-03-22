use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::projections::{Mutation, apply};
use crate::modules::tags::use_cases::list_tags::projection::{ListTagsState, SCHEMA_VERSION};
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

pub struct ListTagsProjector<TStore>
where
    TStore: ProjectionStore<ListTagsState> + Send + Sync + 'static,
{
    pub name: String,
    pub store: TStore,
    pub event_store: InMemoryEventStore<TagEvent>,
    pub technical_tx: broadcast::Sender<ProjectionTechnicalEvent>,
}

impl<TStore> ListTagsProjector<TStore>
where
    TStore: ProjectionStore<ListTagsState> + Send + Sync + 'static,
{
    pub fn new(
        name: impl Into<String>,
        store: TStore,
        event_store: InMemoryEventStore<TagEvent>,
        technical_tx: broadcast::Sender<ProjectionTechnicalEvent>,
    ) -> Self {
        Self {
            name: name.into(),
            store,
            event_store,
            technical_tx,
        }
    }

    pub async fn run(self, mut receiver: broadcast::Receiver<StoredEvent<TagEvent>>) {
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

    async fn apply_stored_event(&self, stored_event: &StoredEvent<TagEvent>) -> anyhow::Result<()> {
        let mut state = self.store.state().await?.unwrap_or_default();
        for mutation in apply(
            &stored_event.stream_id,
            stored_event.stream_version,
            &stored_event.event,
        ) {
            match mutation {
                Mutation::Upsert(row) => {
                    state.rows.insert(row.tag_id.clone(), row);
                }
                Mutation::MarkDeleted {
                    tag_id,
                    deleted_at: _,
                    deleted_by: _,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&tag_id) {
                        row.deleted = true;
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetName {
                    tag_id,
                    name,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&tag_id) {
                        row.name = name;
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetColor {
                    tag_id,
                    color,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&tag_id) {
                        row.color = color;
                        row.last_event_id = Some(last_event_id);
                    }
                }
                Mutation::SetDescription {
                    tag_id,
                    description,
                    last_event_id,
                } => {
                    if let Some(row) = state.rows.get_mut(&tag_id) {
                        row.description = description;
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
mod list_tags_projector_tests {
    use super::*;
    use crate::modules::tags::use_cases::create_tag::command::CreateTag;
    use crate::modules::tags::use_cases::create_tag::handler::CreateTagHandler;
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use rstest::rstest;

    async fn create_one_tag(event_store: InMemoryEventStore<TagEvent>) {
        CreateTagHandler::new(event_store)
            .handle(
                "Tag-t1",
                CreateTag {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Work".to_string(),
                    color: "#FFB3BA".to_string(),
                    description: None,
                    created_at: 1000,
                    created_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_rebuild_and_apply_on_schema_mismatch() {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_one_tag(event_store.clone()).await;

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        // Use a pre-closed channel so the projector exits after rebuild
        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        drop(closed_tx);
        let projector = ListTagsProjector::new("p", projection_store.clone(), event_store, tech_tx);
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
        let (tx, _) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        let event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tx.clone());
        create_one_tag(event_store.clone()).await;

        let mut projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        projection_store.toggle_offline();

        let receiver = tx.subscribe();
        drop(tx);

        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTagsProjector::new("p", projection_store, event_store, tech_tx);
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
        let (tx, _) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        let event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector =
            ListTagsProjector::new("p", projection_store.clone(), event_store.clone(), tech_tx);
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        create_one_tag(event_store.clone()).await;

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
        let (tx, _) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        let event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();
        projection_store
            .save(ListTagsState::default(), 999)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(16);
        let projector =
            ListTagsProjector::new("p", projection_store.clone(), event_store.clone(), tech_tx);
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        create_one_tag(event_store.clone()).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert!(state.rows.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_continue_when_apply_stored_event_fails() {
        let (tx, _) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        let event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tx.clone());

        let mut projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel(16);
        let projector =
            ListTagsProjector::new("p", projection_store.clone(), event_store.clone(), tech_tx);
        let receiver = tx.subscribe();
        tokio::spawn(projector.run(receiver));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        projection_store.toggle_offline();

        create_one_tag(event_store.clone()).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        projection_store.toggle_offline();
        let cp = projection_store.checkpoint().await.unwrap();
        assert_eq!(cp, 0);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_trigger_rebuild_on_lagged_receiver() {
        let (tx, _) = broadcast::channel::<StoredEvent<TagEvent>>(1);
        let event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (lag_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(1);

        create_one_tag(event_store.clone()).await;
        let tag2_handler = CreateTagHandler::new(event_store.clone());
        tag2_handler
            .handle(
                "Tag-t2",
                CreateTag {
                    tag_id: "t2".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Personal".to_string(),
                    color: "#BAFFED".to_string(),
                    description: None,
                    created_at: 2000,
                    created_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();

        let dummy = event_store.load_all_from(0).await.unwrap().remove(0);
        lag_tx.send(dummy.clone()).unwrap();
        lag_tx.send(dummy).unwrap();
        drop(lag_tx);

        let (tech_tx, mut tech_rx) = broadcast::channel(32);
        let projector = ListTagsProjector::new("p", projection_store.clone(), event_store, tech_tx);
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
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_one_tag(event_store.clone()).await;

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        // Save schema version so the initial check passes (no startup rebuild)
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        // Cause lag: capacity-1 channel with 2 events pre-loaded
        let (lag_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(1);
        let dummy = event_store.load_all_from(0).await.unwrap().remove(0);
        lag_tx.send(dummy.clone()).unwrap();
        lag_tx.send(dummy).unwrap();
        drop(lag_tx);

        // Toggle event_store offline so the rebuild triggered by lag fails
        event_store.toggle_offline();

        let (tech_tx, mut tech_rx) = broadcast::channel(32);
        let projector = ListTagsProjector::new("p", projection_store, event_store, tech_tx);
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
    async fn it_should_apply_set_name_set_color_set_description_and_delete_mutations() {
        use crate::modules::tags::use_cases::delete_tag::command::DeleteTag;
        use crate::modules::tags::use_cases::delete_tag::handler::DeleteTagHandler;
        use crate::modules::tags::use_cases::set_tag_color::command::SetTagColor;
        use crate::modules::tags::use_cases::set_tag_color::handler::SetTagColorHandler;
        use crate::modules::tags::use_cases::set_tag_description::command::SetTagDescription;
        use crate::modules::tags::use_cases::set_tag_description::handler::SetTagDescriptionHandler;
        use crate::modules::tags::use_cases::set_tag_name::command::SetTagName;
        use crate::modules::tags::use_cases::set_tag_name::handler::SetTagNameHandler;

        let event_store = InMemoryEventStore::<TagEvent>::new();
        // Create a second tag for deletion so Delete mutation is exercised
        CreateTagHandler::new(event_store.clone())
            .handle(
                "Tag-t2",
                CreateTag {
                    tag_id: "t2".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Temp".to_string(),
                    color: "#FFFFBA".to_string(),
                    description: None,
                    created_at: 500,
                    created_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();
        create_one_tag(event_store.clone()).await;

        SetTagNameHandler::new(event_store.clone())
            .handle(
                "Tag-t1",
                SetTagName {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Renamed".to_string(),
                    set_at: 2000,
                    set_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();

        SetTagColorHandler::new(event_store.clone())
            .handle(
                "Tag-t1",
                SetTagColor {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    color: "#BAFFED".to_string(),
                    set_at: 3000,
                    set_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();

        SetTagDescriptionHandler::new(event_store.clone())
            .handle(
                "Tag-t1",
                SetTagDescription {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    description: Some("My desc".to_string()),
                    set_at: 4000,
                    set_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();

        // Delete t2 to exercise the Mutation::Delete arm
        DeleteTagHandler::new(event_store.clone())
            .handle(
                "Tag-t2",
                DeleteTag {
                    tag_id: "t2".to_string(),
                    tenant_id: "ten1".to_string(),
                    deleted_at: 5000,
                    deleted_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        let (tech_tx, _) = broadcast::channel(16);
        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        drop(closed_tx);

        let projector = ListTagsProjector::new("p", projection_store.clone(), event_store, tech_tx);
        projector.run(receiver).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert!(
            state.rows.get("t2").unwrap().deleted,
            "t2 should be marked deleted"
        );
        let row = state.rows.get("t1").unwrap();
        assert_eq!(row.name, "Renamed");
        assert_eq!(row.color, "#BAFFED");
        assert_eq!(row.description, Some("My desc".to_string()));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_rebuild_when_apply_stored_event_errors() {
        // Covers: line 126 (apply_stored_event?.await? error path) and
        //         line 188 (store.save().await? error path inside apply_stored_event)
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_one_tag(event_store.clone()).await;

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        // No schema_version set → mismatch → rebuild will be triggered.
        // Set fail_next_save so save() inside apply_stored_event fails.
        projection_store.set_fail_next_save();

        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        drop(closed_tx);
        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTagsProjector::new("p", projection_store, event_store, tech_tx);
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
        // Covers: line 128 (store.save_schema_version().await? error path in rebuild)
        // Use an empty event store so the for loop does not run, then save_schema_version fails.
        let event_store = InMemoryEventStore::<TagEvent>::new();

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        // No schema_version set → mismatch → rebuild triggered.
        projection_store.set_fail_next_save_schema_version();

        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        drop(closed_tx);
        let (tech_tx, mut tech_rx) = broadcast::channel(16);
        let projector = ListTagsProjector::new("p", projection_store, event_store, tech_tx);
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
    async fn it_should_mark_deleted_tag_as_deleted_and_keep_in_projection() {
        // Covers: MarkDeleted mutation — deleted tag stays in projection with deleted: true.
        use crate::modules::tags::core::events::v1::tag_color_set::TagColorSetV1;
        use crate::modules::tags::core::events::v1::tag_description_set::TagDescriptionSetV1;
        use crate::modules::tags::core::events::v1::tag_name_set::TagNameSetV1;
        use crate::modules::tags::use_cases::delete_tag::command::DeleteTag;
        use crate::modules::tags::use_cases::delete_tag::handler::DeleteTagHandler;
        use crate::shared::infrastructure::event_store::EventStore;

        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_one_tag(event_store.clone()).await;
        // Delete the tag so its row is removed from the projection state.
        DeleteTagHandler::new(event_store.clone())
            .handle(
                "Tag-t1",
                DeleteTag {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    deleted_at: 2000,
                    deleted_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();

        // Manually append SetName, SetColor, SetDescription events for the now-absent tag.
        // The projection apply_stored_event will hit the None branch for each.
        event_store
            .append(
                "Tag-t1",
                2,
                &[TagEvent::TagNameSetV1(TagNameSetV1 {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Ghost".to_string(),
                    set_at: 3000,
                    set_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();
        event_store
            .append(
                "Tag-t1",
                3,
                &[TagEvent::TagColorSetV1(TagColorSetV1 {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    color: "#BAFFED".to_string(),
                    set_at: 4000,
                    set_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();
        event_store
            .append(
                "Tag-t1",
                4,
                &[TagEvent::TagDescriptionSetV1(TagDescriptionSetV1 {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    description: Some("ghost desc".to_string()),
                    set_at: 5000,
                    set_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        drop(closed_tx);
        let (tech_tx, _) = broadcast::channel(16);
        let projector = ListTagsProjector::new("p", projection_store.clone(), event_store, tech_tx);
        projector.run(receiver).await;

        // Tag t1 was deleted, so it should remain in the projection but marked as deleted.
        let state = projection_store.state().await.unwrap().unwrap();
        assert!(state.rows.get("t1").unwrap().deleted);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_ignore_mutations_for_unknown_tag_id() {
        // Covers the None fallthrough of each `if let Some(row) = state.rows.get_mut(&tag_id)`
        // for MarkDeleted, SetName, SetColor, SetDescription when the tag was never projected.
        use crate::modules::tags::core::events::v1::tag_color_set::TagColorSetV1;
        use crate::modules::tags::core::events::v1::tag_deleted::TagDeletedV1;
        use crate::modules::tags::core::events::v1::tag_description_set::TagDescriptionSetV1;
        use crate::modules::tags::core::events::v1::tag_name_set::TagNameSetV1;
        use crate::shared::infrastructure::event_store::EventStore;

        let event_store = InMemoryEventStore::<TagEvent>::new();
        // Append events for "ghost-id" which is never created — projector will hit None for each mutation.
        event_store
            .append(
                "Tag-ghost",
                0,
                &[TagEvent::TagDeletedV1(TagDeletedV1 {
                    tag_id: "ghost-id".to_string(),
                    tenant_id: "ten1".to_string(),
                    deleted_at: 1000,
                    deleted_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();
        event_store
            .append(
                "Tag-ghost",
                1,
                &[TagEvent::TagNameSetV1(TagNameSetV1 {
                    tag_id: "ghost-id".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Ghost".to_string(),
                    set_at: 2000,
                    set_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();
        event_store
            .append(
                "Tag-ghost",
                2,
                &[TagEvent::TagColorSetV1(TagColorSetV1 {
                    tag_id: "ghost-id".to_string(),
                    tenant_id: "ten1".to_string(),
                    color: "#AABBCC".to_string(),
                    set_at: 3000,
                    set_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();
        event_store
            .append(
                "Tag-ghost",
                3,
                &[TagEvent::TagDescriptionSetV1(TagDescriptionSetV1 {
                    tag_id: "ghost-id".to_string(),
                    tenant_id: "ten1".to_string(),
                    description: Some("ghost".to_string()),
                    set_at: 4000,
                    set_by: "u1".to_string(),
                })],
            )
            .await
            .unwrap();

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        drop(closed_tx);
        let (tech_tx, _) = broadcast::channel(16);
        let projector = ListTagsProjector::new("p", projection_store.clone(), event_store, tech_tx);
        projector.run(receiver).await;

        // All mutations silently ignored — projection remains empty.
        let state = projection_store.state().await.unwrap().unwrap();
        assert!(state.rows.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_exit_when_channel_closed() {
        let (tx, _) = broadcast::channel::<StoredEvent<TagEvent>>(16);
        let event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tx.clone());

        let projection_store = InMemoryProjectionStore::<ListTagsState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (closed_tx, receiver) = broadcast::channel::<StoredEvent<TagEvent>>(1);
        drop(closed_tx);

        let (tech_tx, _) = broadcast::channel(4);
        let projector = ListTagsProjector::new("p", projection_store, event_store, tech_tx);
        projector.run(receiver).await;
    }
}
