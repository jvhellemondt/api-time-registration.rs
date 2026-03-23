use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::events::v1::time_entry_end_set::TimeEntryEndSetV1;
use crate::modules::time_entries::core::events::v1::time_entry_initiated::TimeEntryInitiatedV1;
use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
use crate::modules::time_entries::core::intents::TimeEntryIntent;
use crate::modules::time_entries::core::state::TimeEntryState;
use crate::modules::time_entries::use_cases::set_ended_at::command::SetEndedAt;
use crate::modules::time_entries::use_cases::set_ended_at::decision::{DecideError, Decision};

pub fn decide_set_ended_at(state: &TimeEntryState, command: SetEndedAt) -> Decision {
    let end_set_event = TimeEntryEvent::TimeEntryEndSetV1(TimeEntryEndSetV1 {
        time_entry_id: command.time_entry_id.clone(),
        ended_at: command.ended_at,
        updated_at: command.updated_at,
        updated_by: command.updated_by.clone(),
    });

    match state {
        TimeEntryState::None => {
            let initiated = TimeEntryEvent::TimeEntryInitiatedV1(TimeEntryInitiatedV1 {
                time_entry_id: command.time_entry_id,
                user_id: command.user_id,
                created_at: command.updated_at,
                created_by: command.updated_by,
            });
            Decision::Accepted {
                events: vec![initiated, end_set_event],
                intents: vec![],
            }
        }
        TimeEntryState::Draft {
            started_at: None, ..
        } => Decision::Accepted {
            events: vec![end_set_event],
            intents: vec![],
        },
        TimeEntryState::Draft {
            started_at: Some(s),
            ..
        } => {
            if command.ended_at <= *s {
                return Decision::Rejected {
                    reason: DecideError::InvalidInterval,
                };
            }
            let registered = TimeEntryEvent::TimeEntryRegisteredV1(TimeEntryRegisteredV1 {
                time_entry_id: command.time_entry_id.clone(),
                occurred_at: command.updated_at,
            });
            Decision::Accepted {
                events: vec![end_set_event, registered],
                intents: vec![TimeEntryIntent::PublishTimeEntryRegistered {
                    time_entry_id: command.time_entry_id,
                    occurred_at: command.updated_at,
                }],
            }
        }
        TimeEntryState::Registered { started_at, .. } => {
            if command.ended_at <= *started_at {
                return Decision::Rejected {
                    reason: DecideError::InvalidInterval,
                };
            }
            Decision::Accepted {
                events: vec![end_set_event],
                intents: vec![],
            }
        }
    }
}

#[cfg(test)]
mod decide_set_ended_at_tests {
    use super::*;
    use crate::tests::fixtures::commands::set_ended_at::SetEndedAtBuilder;
    use rstest::{fixture, rstest};

    #[fixture]
    fn command() -> SetEndedAt {
        SetEndedAtBuilder::new().build()
    }

    #[rstest]
    fn it_should_emit_initiated_and_end_set_when_none(command: SetEndedAt) {
        let decision = decide_set_ended_at(&TimeEntryState::None, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 2);
                assert!(matches!(
                    &events[0],
                    TimeEntryEvent::TimeEntryInitiatedV1(_)
                ));
                assert!(matches!(&events[1], TimeEntryEvent::TimeEntryEndSetV1(_)));
                assert!(intents.is_empty());
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_emit_end_set_when_draft_with_no_started_at(command: SetEndedAt) {
        let state = TimeEntryState::Draft {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: None,
            ended_at: None,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_ended_at(&state, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 1);
                assert!(matches!(&events[0], TimeEntryEvent::TimeEntryEndSetV1(_)));
                assert!(intents.is_empty());
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_emit_end_set_and_registered_when_draft_with_started_at() {
        let command = SetEndedAtBuilder::new().ended_at(2_000).build();
        let state = TimeEntryState::Draft {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: Some(1_000),
            ended_at: None,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_ended_at(&state, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 2);
                assert!(matches!(&events[0], TimeEntryEvent::TimeEntryEndSetV1(_)));
                assert!(matches!(
                    &events[1],
                    TimeEntryEvent::TimeEntryRegisteredV1(_)
                ));
                assert_eq!(intents.len(), 1);
                assert!(matches!(
                    &intents[0],
                    TimeEntryIntent::PublishTimeEntryRegistered { .. }
                ));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_reject_invalid_interval_when_draft_with_started_at() {
        let command = SetEndedAtBuilder::new().ended_at(500).build();
        let state = TimeEntryState::Draft {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: Some(1_000),
            ended_at: None,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_ended_at(&state, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::InvalidInterval
            }
        ));
    }

    #[rstest]
    fn it_should_emit_end_set_when_registered(command: SetEndedAt) {
        let state = TimeEntryState::Registered {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: command.ended_at - 100_000,
            ended_at: command.ended_at,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_ended_at(&state, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 1);
                assert!(matches!(&events[0], TimeEntryEvent::TimeEntryEndSetV1(_)));
                assert!(intents.is_empty());
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_reject_invalid_interval_when_registered() {
        let command = SetEndedAtBuilder::new().ended_at(500).build();
        let state = TimeEntryState::Registered {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: 1_000,
            ended_at: 2_000,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_ended_at(&state, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::InvalidInterval
            }
        ));
    }

    #[rstest]
    fn it_should_reject_when_ended_at_equals_started_at() {
        let command = SetEndedAtBuilder::new().ended_at(1_000).build();
        let state = TimeEntryState::Registered {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: 1_000,
            ended_at: 2_000,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_ended_at(&state, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::InvalidInterval
            }
        ));
    }
}
