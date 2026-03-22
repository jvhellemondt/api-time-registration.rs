use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::events::v1::tag_created::TagCreatedV1;
use crate::modules::tags::core::state::TagState;
use crate::modules::tags::use_cases::create_tag::command::CreateTag;
use crate::modules::tags::use_cases::create_tag::decision::{DecideError, Decision};

pub fn decide_create(state: &TagState, command: CreateTag) -> Decision {
    match state {
        TagState::None => Decision::Accepted {
            events: vec![TagEvent::TagCreatedV1(TagCreatedV1 {
                tag_id: command.tag_id,
                tenant_id: command.tenant_id,
                name: command.name,
                color: command.color,
                description: command.description,
                created_at: command.created_at,
                created_by: command.created_by,
            })],
        },
        TagState::Created { .. } | TagState::Deleted { .. } => Decision::Rejected {
            reason: DecideError::TagAlreadyExists,
        },
    }
}

#[cfg(test)]
mod create_tag_decide_tests {
    use super::*;
    use crate::modules::tags::core::events::v1::tag_created::TagCreatedV1;
    use crate::modules::tags::core::evolve::evolve;
    use rstest::{fixture, rstest};

    #[fixture]
    fn command() -> CreateTag {
        CreateTag {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            created_at: 1000,
            created_by: "u1".to_string(),
        }
    }

    #[rstest]
    fn none_state_accepts_create(command: CreateTag) {
        let decision = decide_create(&TagState::None, command);
        match decision {
            Decision::Accepted { events } => {
                assert_eq!(events.len(), 1);
                assert!(matches!(&events[0], TagEvent::TagCreatedV1(_)));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn created_state_rejects_create(command: CreateTag) {
        let created = evolve(
            TagState::None,
            TagEvent::TagCreatedV1(TagCreatedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                name: "Work".to_string(),
                color: "#FFB3BA".to_string(),
                description: None,
                created_at: 1000,
                created_by: "u1".to_string(),
            }),
        );
        let decision = decide_create(&created, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::TagAlreadyExists
            }
        ));
    }

    #[rstest]
    fn deleted_state_rejects_create(command: CreateTag) {
        use crate::modules::tags::core::events::v1::tag_deleted::TagDeletedV1;
        let created = evolve(
            TagState::None,
            TagEvent::TagCreatedV1(TagCreatedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                name: "Work".to_string(),
                color: "#FFB3BA".to_string(),
                description: None,
                created_at: 1000,
                created_by: "u1".to_string(),
            }),
        );
        let deleted = evolve(
            created,
            TagEvent::TagDeletedV1(TagDeletedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                deleted_at: 2000,
                deleted_by: "u1".to_string(),
            }),
        );
        let decision = decide_create(&deleted, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::TagAlreadyExists
            }
        ));
    }
}
