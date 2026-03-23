pub struct SetTimeEntryTags {
    pub time_entry_id: String,
    pub user_id: String,
    pub tag_ids: Vec<String>,
    pub updated_at: i64,
    pub updated_by: String,
}
