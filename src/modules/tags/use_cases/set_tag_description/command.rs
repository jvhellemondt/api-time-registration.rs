#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTagDescription {
    pub tag_id: String,
    pub tenant_id: String,
    pub description: Option<String>,
    pub set_at: i64,
    pub set_by: String,
}
