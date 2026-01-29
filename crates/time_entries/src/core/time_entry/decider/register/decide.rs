// Pure decision function for registration.
//
// Purpose
// - Validate the command against the current state and produce domain events on success.
//
// Responsibilities
// - Enforce rules: end time must be after start time, tag count must be within limits.
// - If state is None, emit TimeEntryRegisteredV1. If state is already registered, return an error.
// - Never perform input or output.

use crate::core::time_entry::{
    decider::register::command::RegisterTimeEntry,
    event::{v1::time_entry_registered::TimeEntryRegisteredV1, TimeEntryEvent},
    state::TimeEntryState,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecideError {
    #[error("time entry already exists")]
    AlreadyExists,
    #[error("end time must be after start time")]
    InvalidInterval,
}

pub fn decide_register(
    state: &TimeEntryState,
    command: RegisterTimeEntry,
) -> Result<Vec<TimeEntryEvent>, DecideError> {
    match state {
        TimeEntryState::None => {
            if command.end_time <= command.start_time {
                return Err(DecideError::InvalidInterval);
            }
            let event = TimeEntryRegisteredV1 {
                time_entry_id: command.time_entry_id,
                user_id: command.user_id,
                start_time: command.start_time,
                end_time: command.end_time,
                tags: command.tags,
                description: command.description,
                created_at: command.created_at,
                created_by: command.created_by,
            };
            Ok(vec![TimeEntryEvent::TimeEntryRegisteredV1(event)])
        }
        _ => Err(DecideError::AlreadyExists),
    }
}

#[cfg(test)]
mod time_entry_register_decide_tests {
    use super::*;
    use crate::core::time_entry::evolve::evolve;
    use rstest::{fixture, rstest};
    use crate::test_support::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;

    #[fixture]
    pub fn register_command() -> RegisterTimeEntry {
        RegisterTimeEntryBuilder::new().build()
    }

    #[fixture]
    fn register_decision_result(
        register_command: RegisterTimeEntry,
    ) -> Result<Vec<TimeEntryEvent>, DecideError> {
        let state = TimeEntryState::None;
        decide_register(&state, register_command)
    }

    #[rstest]
    fn it_should_decide_to_register_the_time_entry(
        register_command: RegisterTimeEntry,
        register_decision_result: Result<Vec<TimeEntryEvent>, DecideError>,
    ) {
        assert!(register_decision_result.is_ok());
        let decision = register_decision_result.unwrap();
        assert_eq!(decision.len(), 1);
        assert_eq!(
            decision,
            vec![TimeEntryEvent::TimeEntryRegisteredV1(
                TimeEntryRegisteredV1 {
                    time_entry_id: register_command.time_entry_id,
                    user_id: register_command.user_id,
                    start_time: register_command.start_time,
                    end_time: register_command.end_time,
                    tags: register_command.tags,
                    description: register_command.description,
                    created_at: register_command.created_at,
                    created_by: register_command.created_by,
                }
            )]
        )
    }

    #[rstest]
    fn it_should_decide_that_the_time_entry_already_exists(
        register_command: RegisterTimeEntry,
        register_decision_result: Result<Vec<TimeEntryEvent>, DecideError>,
    ) {
        let state = TimeEntryState::None;
        let register_event = register_decision_result.unwrap()[0].to_owned();
        let registered_state = evolve(state, register_event);
        let decision = decide_register(&registered_state, register_command);
        assert_eq!(decision, Err(DecideError::AlreadyExists));
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
        assert_eq!(decision, Err(DecideError::InvalidInterval));
    }
}
