use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::state::TimeEntryState;

pub fn evolve(state: TimeEntryState, event: TimeEntryEvent) -> TimeEntryState {
    match (state, event) {
        (TimeEntryState::None, TimeEntryEvent::TimeEntryRegisteredV1(e)) => {
            TimeEntryState::Registered {
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
            }
        }
        (state, _) => state,
    }
}

#[cfg(test)]
mod time_entry_evolve_tests {
    use super::*;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};

    #[fixture]
    fn registered_event() -> TimeEntryRegisteredV1 {
        make_time_entry_registered_v1_event()
    }

    #[rstest]
    fn it_should_evolve_the_state_to_registered(registered_event: TimeEntryRegisteredV1) {
        let state = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()),
        );
        match state {
            TimeEntryState::Registered {
                time_entry_id,
                user_id,
                start_time,
                end_time,
                tags,
                description,
                created_at,
                created_by,
                updated_at,
                updated_by,
                deleted_at,
                last_event_id,
            } => {
                assert_eq!(time_entry_id, registered_event.time_entry_id);
                assert_eq!(user_id, registered_event.user_id);
                assert_eq!(start_time, registered_event.start_time);
                assert_eq!(end_time, registered_event.end_time);
                assert_eq!(tags, registered_event.tags);
                assert_eq!(description, registered_event.description);
                assert_eq!(created_at, registered_event.created_at);
                assert_eq!(created_by, registered_event.created_by);
                assert_eq!(updated_at, registered_event.created_at);
                assert_eq!(updated_by, registered_event.created_by);
                assert_eq!(deleted_at, None);
                assert_eq!(last_event_id, None);
            }
            _ => panic!("expected Registered state"),
        }
    }

    #[rstest]
    fn it_should_not_change_on_duplicate_registered_event(registered_event: TimeEntryRegisteredV1) {
        let registered = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()),
        );
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
        assert_eq!(
            format!("{:?}", next),
            format!("{:?}", registered),
            "state should be unchanged by fallback arm"
        );
    }
}
