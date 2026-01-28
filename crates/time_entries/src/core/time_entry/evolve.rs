// Evolve function: combine a prior state with a new event to produce the next state.
//
// Purpose
// - Define deterministic transitions for each event.
//
// Boundaries
// - No input or output. No side effects.
//
// Testing guidance
// - Given a sequence of events, folding them should yield an expected state.
// - Re-applying the same event should not apply twice.

use crate::core::time_entry::event::TimeEntryEvent;
use crate::core::time_entry::state::TimeEntryState;

pub fn evolve(state: TimeEntryState, event: TimeEntryEvent) -> TimeEntryState {
    match (state, event) {
        (
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryRegisteredV1(e),
        ) => TimeEntryState::Registered {
            time_entry_id: e.time_entry_id,
            user_id: e.user_id,
            start_time: e.start_time,
            end_time: e.end_time,
            tags: e.tags,
            description: e.description,
            created_at: e.created_at,
            created_by: e.created_by.clone(),
            updated_at: e.created_at,
            updated_by: e.created_by,
            deleted_at: None,
            last_event_id: None,
        },
        (state, _) => state,
    }
}

#[cfg(test)]
mod time_entry_evolve_tests {
    use super::*;
    use rstest::{rstest, fixture};
    use crate::core::time_entry::event::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::test_fixtures::make_time_entry_registered_v1_event;

    #[fixture]
    fn registered_event() -> TimeEntryRegisteredV1 {
        make_time_entry_registered_v1_event()
    }

    #[rstest]
    fn it_should_evolve_the_state_to_registered(registered_event: TimeEntryRegisteredV1) {
        let state = evolve(TimeEntryState::None, TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()));

        match state {
            TimeEntryState::Registered {
                time_entry_id: te,
                user_id: u,
                start_time: s,
                end_time: e,
                tags: t,
                description: d,
                created_at: ca,
                created_by: cb,
                updated_at: ua,
                updated_by: ub,
                deleted_at: del,
                last_event_id: le,
            } => {
                assert_eq!(te, registered_event.time_entry_id);
                assert_eq!(u, registered_event.user_id);
                assert_eq!(s, registered_event.start_time);
                assert_eq!(e, registered_event.end_time);
                assert_eq!(t, registered_event.tags);
                assert_eq!(d, registered_event.description);
                assert_eq!(ca, registered_event.created_at);
                assert_eq!(cb, registered_event.created_by);
                assert_eq!(ua, registered_event.created_at);
                assert_eq!(ub, registered_event.created_by);
                assert_eq!(del, None);
                assert_eq!(le, None);
            }
            _ => panic!("expected to evolve to Registered state"),
        }
    }

    #[rstest]
    fn it_should_not_change_on_duplicate_registered_event(registered_event: TimeEntryRegisteredV1) {
        let registered = evolve(TimeEntryState::None, TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()));

        let ev = TimeEntryEvent::TimeEntryRegisteredV1(TimeEntryRegisteredV1 {
            time_entry_id: "te-fixed-0001".into(),
            user_id: "user-fixed-0001".into(),
            start_time: 1_700_000_000_000,
            end_time: 1_700_000_360_000,
            tags: vec!["Work".into()],
            description: "This is a test".into(),
            created_at: 1_700_000_000_000,
            created_by: "user-fixed-0001".into(),
        });

        let next = evolve(registered.clone(), ev);
        assert_eq!(format!("{:?}", next), format!("{:?}", registered), "state should be unchanged by fallback arm");
    }
}
