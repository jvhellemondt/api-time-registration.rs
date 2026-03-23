#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TimeEntryTagsSetV1 {
    pub time_entry_id: String,
    pub tag_ids: Vec<String>,
    pub updated_at: i64,
    pub updated_by: String,
}
