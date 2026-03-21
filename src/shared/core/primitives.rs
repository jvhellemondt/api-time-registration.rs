// Bounded context-wide primitive types shared across all modules.
// Add types here only when two or more modules need the same type.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, async_graphql::SimpleObject)]
pub struct Tag {
    pub tag_id: String,
    pub name: String,
    pub color: String,
}
