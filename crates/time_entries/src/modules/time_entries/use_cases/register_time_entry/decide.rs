use crate::modules::time_entries::core::{
    events::{TimeEntryEvent, v1::time_entry_registered::TimeEntryRegisteredV1},
    intents::TimeEntryIntent,
    state::TimeEntryState,
};
use crate::modules::time_entries::use_cases::register_time_entry::{
    command::RegisterTimeEntry,
    decision::{DecideError, Decision},
};

pub fn decide_register(state: &TimeEntryState, command: RegisterTimeEntry) -> Decision {
    match state {
        TimeEntryState::None => {
            if command.end_time <= command.start_time {
                return Decision::Rejected {
                    reason: DecideError::InvalidInterval,
                };
            }
            let payload = TimeEntryRegisteredV1 {
                time_entry_id: command.time_entry_id,
                user_id: command.user_id,
                start_time: command.start_time,
                end_time: command.end_time,
                tags: command.tags,
                description: command.description,
                created_at: command.created_at,
                created_by: command.created_by,
            };
            Decision::Accepted {
                events: vec![TimeEntryEvent::TimeEntryRegisteredV1(payload.clone())],
                intents: vec![TimeEntryIntent::PublishTimeEntryRegistered { payload }],
            }
        }
        _ => Decision::Rejected {
            reason: DecideError::AlreadyExists,
        },
    }
}

#[cfg(test)]
mod time_entry_register_decide_tests {
    use super::*;
    use crate::modules::time_entries::core::evolve::evolve;
    use crate::modules::time_entries::use_cases::register_time_entry::decision::DecideError;
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use rstest::{fixture, rstest};

    #[fixture]
    fn register_command() -> RegisterTimeEntry {
        RegisterTimeEntryBuilder::new().build()
    }

    #[rstest]
    fn it_should_decide_to_register_the_time_entry(register_command: RegisterTimeEntry) {
        let state = TimeEntryState::None;
        let decision = decide_register(&state, register_command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 1);
                assert_eq!(intents.len(), 1);
                assert!(matches!(
                    &events[0],
                    TimeEntryEvent::TimeEntryRegisteredV1(_)
                ));
                assert!(matches!(
                    &intents[0],
                    TimeEntryIntent::PublishTimeEntryRegistered { .. }
                ));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_decide_that_the_time_entry_already_exists(register_command: RegisterTimeEntry) {
        let state = TimeEntryState::None;
        let first = decide_register(&state, register_command.clone());
        let register_event = match first {
            Decision::Accepted { mut events, .. } => events.remove(0),
            _ => panic!("expected Accepted for first decision"),
        };
        let registered_state = evolve(state, register_event);
        let second = decide_register(&registered_state, register_command);
        assert!(matches!(
            second,
            Decision::Rejected {
                reason: DecideError::AlreadyExists
            }
        ));
    }

    #[rstest]
    fn it_should_decide_that_the_time_entry_is_invalid_by_interval(
        register_command: RegisterTimeEntry,
    ) {
        let state = TimeEntryState::None;
        let command = RegisterTimeEntryBuilder::new()
            .start_time(register_command.end_time)
            .end_time(register_command.start_time)
            .build();
        let decision = decide_register(&state, command);
        assert!(matches!(
            decision,
            Decision::Rejected {
                reason: DecideError::InvalidInterval
            }
        ));
    }
}
