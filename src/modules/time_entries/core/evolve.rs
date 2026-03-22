use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::state::TimeEntryState;

pub fn evolve(state: TimeEntryState, event: TimeEntryEvent) -> TimeEntryState {
    match (state, event) {
        (TimeEntryState::None, TimeEntryEvent::TimeEntryInitiatedV1(e)) => TimeEntryState::Draft {
            time_entry_id: e.time_entry_id,
            user_id: e.user_id,
            started_at: None,
            ended_at: None,
            created_at: e.created_at,
            created_by: e.created_by,
        },
        (
            TimeEntryState::Draft {
                time_entry_id,
                user_id,
                ended_at,
                created_at,
                created_by,
                ..
            },
            TimeEntryEvent::TimeEntryStartSetV1(e),
        ) => TimeEntryState::Draft {
            time_entry_id,
            user_id,
            started_at: Some(e.started_at),
            ended_at,
            created_at,
            created_by,
        },
        (
            TimeEntryState::Draft {
                time_entry_id,
                user_id,
                started_at,
                created_at,
                created_by,
                ..
            },
            TimeEntryEvent::TimeEntryEndSetV1(e),
        ) => TimeEntryState::Draft {
            time_entry_id,
            user_id,
            started_at,
            ended_at: Some(e.ended_at),
            created_at,
            created_by,
        },
        (
            TimeEntryState::Draft {
                time_entry_id,
                user_id,
                started_at,
                ended_at,
                created_at,
                created_by,
            },
            TimeEntryEvent::TimeEntryRegisteredV1(_),
        ) => TimeEntryState::Registered {
            time_entry_id,
            user_id,
            started_at: started_at.unwrap_or(0),
            ended_at: ended_at.unwrap_or(0),
            created_at,
            created_by,
        },
        (
            TimeEntryState::Registered {
                time_entry_id,
                user_id,
                ended_at,
                created_at,
                created_by,
                ..
            },
            TimeEntryEvent::TimeEntryStartSetV1(e),
        ) => TimeEntryState::Registered {
            time_entry_id,
            user_id,
            started_at: e.started_at,
            ended_at,
            created_at,
            created_by,
        },
        (
            TimeEntryState::Registered {
                time_entry_id,
                user_id,
                started_at,
                created_at,
                created_by,
                ..
            },
            TimeEntryEvent::TimeEntryEndSetV1(e),
        ) => TimeEntryState::Registered {
            time_entry_id,
            user_id,
            started_at,
            ended_at: e.ended_at,
            created_at,
            created_by,
        },
        (state, _) => state,
    }
}

#[cfg(test)]
mod time_entry_evolve_tests {
    use super::*;
    use crate::modules::time_entries::core::events::v1::time_entry_end_set::TimeEntryEndSetV1;
    use crate::modules::time_entries::core::events::v1::time_entry_initiated::TimeEntryInitiatedV1;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::modules::time_entries::core::events::v1::time_entry_start_set::TimeEntryStartSetV1;
    use rstest::rstest;

    fn make_initiated() -> TimeEntryInitiatedV1 {
        TimeEntryInitiatedV1 {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            created_at: 1_000,
            created_by: "user-0001".to_string(),
        }
    }

    fn make_start_set(started_at: i64) -> TimeEntryStartSetV1 {
        TimeEntryStartSetV1 {
            time_entry_id: "te-0001".to_string(),
            started_at,
            updated_at: 1_000,
            updated_by: "user-0001".to_string(),
        }
    }

    fn make_end_set(ended_at: i64) -> TimeEntryEndSetV1 {
        TimeEntryEndSetV1 {
            time_entry_id: "te-0001".to_string(),
            ended_at,
            updated_at: 1_000,
            updated_by: "user-0001".to_string(),
        }
    }

    fn make_registered() -> TimeEntryRegisteredV1 {
        TimeEntryRegisteredV1 {
            time_entry_id: "te-0001".to_string(),
            occurred_at: 1_000,
        }
    }

    #[rstest]
    fn none_plus_initiated_becomes_draft() {
        let state = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryInitiatedV1(make_initiated()),
        );
        match state {
            TimeEntryState::Draft {
                time_entry_id,
                user_id,
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(time_entry_id, "te-0001");
                assert_eq!(user_id, "user-0001");
                assert_eq!(started_at, None);
                assert_eq!(ended_at, None);
            }
            _ => panic!("expected Draft"),
        }
    }

    #[rstest]
    fn draft_plus_start_set_updates_started_at() {
        let draft = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryInitiatedV1(make_initiated()),
        );
        let state = evolve(
            draft,
            TimeEntryEvent::TimeEntryStartSetV1(make_start_set(500)),
        );
        match state {
            TimeEntryState::Draft {
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(started_at, Some(500));
                assert_eq!(ended_at, None);
            }
            _ => panic!("expected Draft"),
        }
    }

    #[rstest]
    fn draft_plus_end_set_updates_ended_at() {
        let draft = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryInitiatedV1(make_initiated()),
        );
        let state = evolve(draft, TimeEntryEvent::TimeEntryEndSetV1(make_end_set(800)));
        match state {
            TimeEntryState::Draft {
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(started_at, None);
                assert_eq!(ended_at, Some(800));
            }
            _ => panic!("expected Draft"),
        }
    }

    #[rstest]
    fn draft_plus_registered_becomes_registered() {
        let draft = TimeEntryState::Draft {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            started_at: Some(500),
            ended_at: Some(800),
            created_at: 1_000,
            created_by: "user-0001".to_string(),
        };
        let state = evolve(
            draft,
            TimeEntryEvent::TimeEntryRegisteredV1(make_registered()),
        );
        match state {
            TimeEntryState::Registered {
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(started_at, 500);
                assert_eq!(ended_at, 800);
            }
            _ => panic!("expected Registered"),
        }
    }

    #[rstest]
    fn registered_plus_start_set_updates_started_at() {
        let registered = TimeEntryState::Registered {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            started_at: 500,
            ended_at: 800,
            created_at: 1_000,
            created_by: "user-0001".to_string(),
        };
        let state = evolve(
            registered,
            TimeEntryEvent::TimeEntryStartSetV1(make_start_set(600)),
        );
        match state {
            TimeEntryState::Registered {
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(started_at, 600);
                assert_eq!(ended_at, 800);
            }
            _ => panic!("expected Registered"),
        }
    }

    #[rstest]
    fn registered_plus_end_set_updates_ended_at() {
        let registered = TimeEntryState::Registered {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            started_at: 500,
            ended_at: 800,
            created_at: 1_000,
            created_by: "user-0001".to_string(),
        };
        let state = evolve(
            registered,
            TimeEntryEvent::TimeEntryEndSetV1(make_end_set(900)),
        );
        match state {
            TimeEntryState::Registered {
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(started_at, 500);
                assert_eq!(ended_at, 900);
            }
            _ => panic!("expected Registered"),
        }
    }

    #[rstest]
    fn fallback_none_plus_start_set_is_unchanged() {
        let state = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryStartSetV1(make_start_set(500)),
        );
        assert_eq!(state, TimeEntryState::None);
    }

    #[rstest]
    fn fallback_none_plus_registered_is_unchanged() {
        let state = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryRegisteredV1(make_registered()),
        );
        assert_eq!(state, TimeEntryState::None);
    }

    #[rstest]
    fn fallback_registered_plus_initiated_is_unchanged() {
        let registered = TimeEntryState::Registered {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            started_at: 500,
            ended_at: 800,
            created_at: 1_000,
            created_by: "user-0001".to_string(),
        };
        let expected = registered.clone();
        let state = evolve(
            registered,
            TimeEntryEvent::TimeEntryInitiatedV1(make_initiated()),
        );
        assert_eq!(state, expected);
    }
}
