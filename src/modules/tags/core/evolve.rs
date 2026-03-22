use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::state::TagState;

pub fn evolve(state: TagState, event: TagEvent) -> TagState {
    match (state, event) {
        (TagState::None, TagEvent::TagCreatedV1(e)) => TagState::Created {
            tag_id: e.tag_id,
            tenant_id: e.tenant_id,
            name: e.name,
            color: e.color,
            description: e.description,
            created_at: e.created_at,
            created_by: e.created_by,
        },
        (
            TagState::Created {
                tag_id,
                tenant_id,
                name,
                color,
                description,
                ..
            },
            TagEvent::TagDeletedV1(_),
        ) => TagState::Deleted {
            tag_id,
            tenant_id,
            name,
            color,
            description,
        },
        (
            TagState::Created {
                tag_id,
                tenant_id,
                name: _,
                color,
                description,
                created_at,
                created_by,
            },
            TagEvent::TagNameSetV1(e),
        ) => TagState::Created {
            tag_id,
            tenant_id,
            name: e.name,
            color,
            description,
            created_at,
            created_by,
        },
        (
            TagState::Created {
                tag_id,
                tenant_id,
                name,
                color: _,
                description,
                created_at,
                created_by,
            },
            TagEvent::TagColorSetV1(e),
        ) => TagState::Created {
            tag_id,
            tenant_id,
            name,
            color: e.color,
            description,
            created_at,
            created_by,
        },
        (
            TagState::Created {
                tag_id,
                tenant_id,
                name,
                color,
                description: _,
                created_at,
                created_by,
            },
            TagEvent::TagDescriptionSetV1(e),
        ) => TagState::Created {
            tag_id,
            tenant_id,
            name,
            color,
            description: e.description,
            created_at,
            created_by,
        },
        (state, _) => state,
    }
}

#[cfg(test)]
mod tag_evolve_tests {
    use super::*;
    use crate::modules::tags::core::events::v1::tag_color_set::TagColorSetV1;
    use crate::modules::tags::core::events::v1::tag_created::TagCreatedV1;
    use crate::modules::tags::core::events::v1::tag_deleted::TagDeletedV1;
    use crate::modules::tags::core::events::v1::tag_description_set::TagDescriptionSetV1;
    use crate::modules::tags::core::events::v1::tag_name_set::TagNameSetV1;
    use rstest::rstest;

    fn created_event() -> TagCreatedV1 {
        TagCreatedV1 {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            created_at: 1000,
            created_by: "u1".to_string(),
        }
    }

    fn created_state() -> TagState {
        evolve(TagState::None, TagEvent::TagCreatedV1(created_event()))
    }

    #[rstest]
    fn none_plus_tag_created_yields_created_state() {
        let state = created_state();
        match state {
            TagState::Created {
                tag_id,
                name,
                color,
                description,
                ..
            } => {
                assert_eq!(tag_id, "t1");
                assert_eq!(name, "Work");
                assert_eq!(color, "#FFB3BA");
                assert_eq!(description, None);
            }
            _ => panic!("expected Created"),
        }
    }

    #[rstest]
    fn created_plus_tag_deleted_yields_deleted_state() {
        let state = evolve(
            created_state(),
            TagEvent::TagDeletedV1(TagDeletedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                deleted_at: 2000,
                deleted_by: "u1".to_string(),
            }),
        );
        match state {
            TagState::Deleted { tag_id, name, .. } => {
                assert_eq!(tag_id, "t1");
                assert_eq!(name, "Work");
            }
            _ => panic!("expected Deleted"),
        }
    }

    #[rstest]
    fn created_plus_tag_name_set_updates_name() {
        let state = evolve(
            created_state(),
            TagEvent::TagNameSetV1(TagNameSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                name: "Billable".to_string(),
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        match state {
            TagState::Created { name, .. } => assert_eq!(name, "Billable"),
            _ => panic!("expected Created"),
        }
    }

    #[rstest]
    fn created_plus_tag_color_set_updates_color() {
        let state = evolve(
            created_state(),
            TagEvent::TagColorSetV1(TagColorSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                color: "#BAE1FF".to_string(),
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        match state {
            TagState::Created { color, .. } => assert_eq!(color, "#BAE1FF"),
            _ => panic!("expected Created"),
        }
    }

    #[rstest]
    fn created_plus_tag_description_set_updates_description() {
        let state = evolve(
            created_state(),
            TagEvent::TagDescriptionSetV1(TagDescriptionSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                description: Some("Client work".to_string()),
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        match state {
            TagState::Created { description, .. } => {
                assert_eq!(description, Some("Client work".to_string()))
            }
            _ => panic!("expected Created"),
        }
    }

    #[rstest]
    fn created_plus_tag_description_set_to_none_clears_description() {
        let state = evolve(
            created_state(),
            TagEvent::TagDescriptionSetV1(TagDescriptionSetV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                description: None,
                set_at: 2000,
                set_by: "u1".to_string(),
            }),
        );
        match state {
            TagState::Created { description, .. } => assert_eq!(description, None),
            _ => panic!("expected Created"),
        }
    }

    #[rstest]
    fn unmatched_event_on_none_leaves_state_unchanged() {
        let state = evolve(
            TagState::None,
            TagEvent::TagDeletedV1(TagDeletedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                deleted_at: 2000,
                deleted_by: "u1".to_string(),
            }),
        );
        assert!(matches!(state, TagState::None));
    }
}
