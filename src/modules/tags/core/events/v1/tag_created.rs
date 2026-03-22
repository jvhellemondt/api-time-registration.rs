#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TagCreatedV1 {
    pub tag_id: String,
    pub tenant_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod tag_created_event_tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;

    #[fixture]
    fn event() -> TagCreatedV1 {
        TagCreatedV1 {
            tag_id: "tag-fixed-0001".to_string(),
            tenant_id: "tenant-hardcoded".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            created_at: 1700000000000,
            created_by: "user-fixed-0001".to_string(),
        }
    }

    #[rstest]
    fn it_should_have_correct_fields(event: TagCreatedV1) {
        assert_eq!(event.tag_id, "tag-fixed-0001");
        assert_eq!(event.tenant_id, "tenant-hardcoded");
        assert_eq!(event.name, "Work");
        assert_eq!(event.color, "#FFB3BA");
        assert_eq!(event.description, None);
    }

    #[rstest]
    fn it_serializes_stable(event: TagCreatedV1) {
        let golden: serde_json::Value = serde_json::from_str(
            &fs::read_to_string("./src/tests/fixtures/events/json/tag_created_v1.json").unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&event).unwrap(), golden);
    }
}
