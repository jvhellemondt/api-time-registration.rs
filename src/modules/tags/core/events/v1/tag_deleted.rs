#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TagDeletedV1 {
    pub tag_id: String,
    pub tenant_id: String,
    pub deleted_at: i64,
    pub deleted_by: String,
}

#[cfg(test)]
mod tag_deleted_event_tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;

    #[fixture]
    fn event() -> TagDeletedV1 {
        TagDeletedV1 {
            tag_id: "tag-fixed-0001".to_string(),
            tenant_id: "tenant-hardcoded".to_string(),
            deleted_at: 1700000360000,
            deleted_by: "user-fixed-0001".to_string(),
        }
    }

    #[rstest]
    fn it_should_have_correct_fields(event: TagDeletedV1) {
        assert_eq!(event.tag_id, "tag-fixed-0001");
        assert_eq!(event.tenant_id, "tenant-hardcoded");
        assert_eq!(event.deleted_at, 1700000360000);
        assert_eq!(event.deleted_by, "user-fixed-0001");
    }

    #[rstest]
    fn it_serializes_stable(event: TagDeletedV1) {
        let golden: serde_json::Value = serde_json::from_str(
            &fs::read_to_string("./src/tests/fixtures/events/json/tag_deleted_v1.json").unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&event).unwrap(), golden);
    }
}
