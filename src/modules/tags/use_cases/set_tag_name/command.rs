#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTagName {
    pub tag_id: String,
    pub tenant_id: String,
    pub name: String,
    pub set_at: i64,
    pub set_by: String,
}
