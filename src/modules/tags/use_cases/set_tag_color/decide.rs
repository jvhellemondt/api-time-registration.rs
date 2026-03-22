use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::events::v1::tag_color_set::TagColorSetV1;
use crate::modules::tags::core::state::TagState;
use crate::modules::tags::use_cases::set_tag_color::command::SetTagColor;
use crate::modules::tags::use_cases::set_tag_color::decision::{DecideError, Decision};

pub fn decide_set_color(state: &TagState, command: SetTagColor) -> Decision {
    match state {
        TagState::Created {
            tag_id, tenant_id, ..
        } => Decision::Accepted {
            events: vec![TagEvent::TagColorSetV1(TagColorSetV1 {
                tag_id: tag_id.clone(),
                tenant_id: tenant_id.clone(),
                color: command.color,
                set_at: command.set_at,
                set_by: command.set_by,
            })],
        },
        TagState::None => Decision::Rejected {
            reason: DecideError::TagNotFound,
        },
        TagState::Deleted { .. } => Decision::Rejected {
            reason: DecideError::TagDeleted,
        },
    }
}

#[cfg(test)]
mod set_tag_color_decide_tests {
    use super::*;
    use crate::modules::tags::core::events::v1::tag_created::TagCreatedV1;
    use crate::modules::tags::core::events::v1::tag_deleted::TagDeletedV1;
    use crate::modules::tags::core::evolve::evolve;
    use rstest::{fixture, rstest};

    #[fixture]
    fn command() -> SetTagColor {
        SetTagColor {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            color: "#BAE1FF".to_string(),
            set_at: 2000,
            set_by: "u1".to_string(),
        }
    }

    fn created_state() -> TagState {
        evolve(
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
        )
    }

    fn deleted_state() -> TagState {
        evolve(
            created_state(),
            TagEvent::TagDeletedV1(TagDeletedV1 {
                tag_id: "t1".to_string(),
                tenant_id: "ten1".to_string(),
                deleted_at: 2000,
                deleted_by: "u1".to_string(),
            }),
        )
    }

    #[rstest]
    fn created_state_accepts_set_color(command: SetTagColor) {
        let decision = decide_set_color(&created_state(), command);
        match decision {
            Decision::Accepted { events } => {
                assert_eq!(events.len(), 1);
                assert!(matches!(&events[0], TagEvent::TagColorSetV1(_)));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn none_state_rejects_with_tag_not_found(command: SetTagColor) {
        let decision = decide_set_color(&TagState::None, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::TagNotFound
            }
        ));
    }

    #[rstest]
    fn deleted_state_rejects_with_tag_deleted(command: SetTagColor) {
        let decision = decide_set_color(&deleted_state(), command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::TagDeleted
            }
        ));
    }
}
