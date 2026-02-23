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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryView {
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
}

impl From<TimeEntryRow> for TimeEntryView {
    fn from(row: TimeEntryRow) -> Self {
        Self {
            time_entry_id: row.time_entry_id,
            user_id: row.user_id,
            start_time: row.start_time,
            end_time: row.end_time,
            tags: row.tags,
            description: row.description,
            created_at: row.created_at,
            created_by: row.created_by,
            updated_at: row.updated_at,
            updated_by: row.updated_by,
            deleted_at: row.deleted_at,
        }
    }
}

#[cfg(test)]
mod time_entry_projector_model_tests {
    use super::*;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::rstest;

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
    }
}
