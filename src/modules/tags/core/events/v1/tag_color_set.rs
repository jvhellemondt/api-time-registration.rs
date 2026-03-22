#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TagColorSetV1 {
    pub tag_id: String,
    pub tenant_id: String,
    pub color: String,
    pub set_at: i64,
    pub set_by: String,
}

#[cfg(test)]
mod tag_color_set_event_tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;

    #[fixture]
    fn event() -> TagColorSetV1 {
        TagColorSetV1 {
            tag_id: "tag-fixed-0001".to_string(),
            tenant_id: "tenant-hardcoded".to_string(),
            color: "#BAE1FF".to_string(),
            set_at: 1700000360000,
            set_by: "user-fixed-0001".to_string(),
        }
    }

    #[rstest]
    fn it_should_have_correct_fields(event: TagColorSetV1) {
        assert_eq!(event.tag_id, "tag-fixed-0001");
        assert_eq!(event.color, "#BAE1FF");
        assert_eq!(event.set_at, 1700000360000);
    }

    #[rstest]
    fn it_serializes_stable(event: TagColorSetV1) {
        let golden: serde_json::Value = serde_json::from_str(
            &fs::read_to_string("./src/tests/fixtures/events/json/tag_color_set_v1.json").unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&event).unwrap(), golden);
    }
}
