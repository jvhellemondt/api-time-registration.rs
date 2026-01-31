// Read model row for a single time entry and last_event_id for idempotency.
//
// Purpose
// - Represent how a time entry is stored for fast reads in the projection store.
//
// Responsibilities
// - Map from event fields where possible.
// - Include last_event_id so idempotent upserts and patches are possible.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryRow {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
    pub last_event_id: Option<String>,
}

#[cfg(test)]
mod time_entry_projector_model_tests {
    use rstest::rstest;
    use crate::test_support::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use super::*;

    #[rstest]
    fn it_should_create_the_model() {
        let event = make_time_entry_registered_v1_event();
        let model = TimeEntryRow {
            time_entry_id: event.time_entry_id.clone(),
            user_id: event.user_id.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            tags: event.tags.clone(),
            description: event.description.clone(),
            created_at: event.created_at,
            created_by: event.created_by.clone(),
            updated_at: event.created_at,
            updated_by: event.created_by.clone(),
            deleted_at: None,
            last_event_id: None,
        };
        assert_eq!(model.time_entry_id, event.time_entry_id);
        assert_eq!(model.user_id, event.user_id);
        assert_eq!(model.start_time, event.start_time);
        assert_eq!(model.end_time, event.end_time);
        assert_eq!(model.tags, event.tags);
        assert_eq!(model.description, event.description);
        assert_eq!(model.created_at, event.created_at);
        assert_eq!(model.created_by, event.created_by);
        assert_eq!(model.updated_at, event.created_at);
        assert_eq!(model.updated_by, event.created_by);
        assert_eq!(model.deleted_at, None);
        assert_eq!(model.last_event_id, None);
    }
}
