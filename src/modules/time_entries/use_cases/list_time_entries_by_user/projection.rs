pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeEntryStatus {
    Draft,
    Registered,
}

#[derive(Clone, Default)]
pub struct ListTimeEntriesState {
    pub rows: std::collections::HashMap<String, TimeEntryRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryRow {
    pub time_entry_id: String,
    pub user_id: String,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub tag_ids: Vec<String>,
    pub status: TimeEntryStatus,
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
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub tag_ids: Vec<String>,
    pub status: TimeEntryStatus,
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
            started_at: row.started_at,
            ended_at: row.ended_at,
            tag_ids: row.tag_ids,
            status: row.status,
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
    use rstest::rstest;

    #[rstest]
    fn it_should_create_the_draft_row() {
        let row = TimeEntryRow {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            started_at: None,
            ended_at: None,
            tag_ids: vec![],
            status: TimeEntryStatus::Draft,
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
            updated_at: 1_700_000_000_000i64,
            updated_by: "user-fixed-0001".to_string(),
            deleted_at: None,
            last_event_id: None,
        };
        assert_eq!(row.time_entry_id, "te-fixed-0001");
        assert_eq!(row.user_id, "user-fixed-0001");
        assert_eq!(row.status, TimeEntryStatus::Draft);
        assert_eq!(row.started_at, None);
        assert_eq!(row.ended_at, None);
        assert!(row.tag_ids.is_empty());
    }

    #[rstest]
    fn it_should_create_the_registered_row() {
        let row = TimeEntryRow {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            started_at: Some(1_700_000_000_000i64),
            ended_at: Some(1_700_000_360_000i64),
            tag_ids: vec!["tag-1".to_string()],
            status: TimeEntryStatus::Registered,
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
            updated_at: 1_700_000_000_000i64,
            updated_by: "user-fixed-0001".to_string(),
            deleted_at: None,
            last_event_id: None,
        };
        assert_eq!(row.status, TimeEntryStatus::Registered);
        assert_eq!(row.started_at, Some(1_700_000_000_000i64));
        assert_eq!(row.ended_at, Some(1_700_000_360_000i64));
        assert_eq!(row.tag_ids, vec!["tag-1".to_string()]);
    }

    #[rstest]
    fn it_should_convert_row_to_view() {
        let row = TimeEntryRow {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            started_at: Some(1_700_000_000_000i64),
            ended_at: Some(1_700_000_360_000i64),
            tag_ids: vec!["tag-1".to_string()],
            status: TimeEntryStatus::Registered,
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
            updated_at: 1_700_000_000_000i64,
            updated_by: "user-fixed-0001".to_string(),
            deleted_at: None,
            last_event_id: Some("stream:1".to_string()),
        };
        let view = TimeEntryView::from(row.clone());
        assert_eq!(view.time_entry_id, row.time_entry_id);
        assert_eq!(view.started_at, row.started_at);
        assert_eq!(view.ended_at, row.ended_at);
        assert_eq!(view.status, row.status);
        assert_eq!(view.tag_ids, row.tag_ids);
    }

    #[rstest]
    fn it_should_serialize_status_as_lowercase() {
        let draft = TimeEntryStatus::Draft;
        let registered = TimeEntryStatus::Registered;
        assert_eq!(
            serde_json::to_value(&draft).unwrap(),
            serde_json::json!("draft")
        );
        assert_eq!(
            serde_json::to_value(&registered).unwrap(),
            serde_json::json!("registered")
        );
    }
}
