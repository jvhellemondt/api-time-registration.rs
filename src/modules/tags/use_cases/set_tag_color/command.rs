#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTagColor {
    pub tag_id: String,
    pub tenant_id: String,
    pub color: String,
    pub set_at: i64,
    pub set_by: String,
}
