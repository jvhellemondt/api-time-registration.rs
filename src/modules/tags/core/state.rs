#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagState {
    None,
    Created {
        tag_id: String,
        tenant_id: String,
        name: String,
        color: String,
        description: Option<String>,
        created_at: i64,
        created_by: String,
    },
    Deleted {
        tag_id: String,
        tenant_id: String,
        name: String,
        color: String,
        description: Option<String>,
    },
}

#[cfg(test)]
mod tag_state_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn it_should_default_to_none() {
        let state = TagState::None;
        assert!(matches!(state, TagState::None));
    }

    #[rstest]
    fn it_should_hold_created_fields() {
        let state = TagState::Created {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: Some("desc".to_string()),
            created_at: 1000,
            created_by: "u1".to_string(),
        };
        assert!(matches!(state, TagState::Created { .. }));
    }

    #[rstest]
    fn it_should_hold_deleted_fields() {
        let state = TagState::Deleted {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
        };
        assert!(matches!(state, TagState::Deleted { .. }));
    }
}
