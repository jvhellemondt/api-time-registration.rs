#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTag {
    pub tag_id: String,
    pub tenant_id: String,
    pub deleted_at: i64,
    pub deleted_by: String,
}
