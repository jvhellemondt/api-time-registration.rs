pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Default)]
pub struct ListTagsState {
    pub rows: std::collections::HashMap<String, TagRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TagRow {
    pub tag_id: String,
    pub tenant_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub deleted: bool,
    pub last_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TagView {
    pub tag_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub deleted: bool,
}

impl From<TagRow> for TagView {
    fn from(row: TagRow) -> Self {
        Self {
            tag_id: row.tag_id,
            name: row.name,
            color: row.color,
            description: row.description,
            deleted: row.deleted,
        }
    }
}

#[cfg(test)]
mod list_tags_projection_model_tests {
    use super::*;
    use rstest::rstest;

    fn make_row() -> TagRow {
        TagRow {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            deleted: false,
            last_event_id: None,
        }
    }

    #[rstest]
    fn it_should_convert_row_to_view() {
        let row = make_row();
        let view = TagView::from(row.clone());
        assert_eq!(view.tag_id, row.tag_id);
        assert_eq!(view.name, row.name);
        assert_eq!(view.color, row.color);
        assert_eq!(view.description, row.description);
        assert_eq!(view.deleted, false);
    }

    #[rstest]
    fn it_should_convert_row_with_description_to_view() {
        let mut row = make_row();
        row.description = Some("Client work".to_string());
        let view = TagView::from(row);
        assert_eq!(view.description, Some("Client work".to_string()));
    }
}
