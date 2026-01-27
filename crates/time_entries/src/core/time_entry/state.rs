// TimeEntryState is the canonical domain state after folding events.
//
// Suggested structure (to implement later)
// - None
// - Registered { time_entry_id, user_id, start_time, end_time, tags, description, created_at,
//                created_by, updated_at, updated_by, deleted_at, last_event_id }
//
// Boundaries
// - This file must not perform input or output.
// - Keep it framework-free.
//
// Testing guidance
// - Use the evolve function to produce states from events and assert expected fields.

// Purpose
// - Represents the domain state of a time entry after folding events.
// - Encodes lifecycle as explicit variants for safety and clarity.
//
// Notes
// - Choose either epoch seconds or [epoch milliseconds] for all i64 time values and be consistent.
// - Use Option<i64> for deleted_at to avoid sentinel values like 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeEntryState {
    None,
    Registered {
        time_entry_id: String,
        user_id: String,
        start_time: i64,
        end_time: i64,
        tags: Vec<String>,
        description: String,
        created_at: i64,
        created_by: String,
        updated_at: i64,
        updated_by: String,
        deleted_at: Option<i64>,
        last_event_id: Option<String>,
    },
}

#[cfg(test)]
mod time_entry_state_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn it_should_create_the_blank_state() {
        let state = TimeEntryState::None;

        match state {
            TimeEntryState::None => assert!(true),
            _ => panic!("expected None state"),
        }
    }

    #[rstest]
    fn it_should_create_the_registered_state() {
        let time_entry_id = "te-fixed-0001".to_string();
        let user_id = "user-fixed-0001".to_string();
        let start_time = 1_700_000_000_000i64;
        let end_time = 1_700_000_360_000i64;
        let tags = vec!["Work".to_string()];
        let description = "This is a test".to_string();
        let created_at = 1_700_000_000_000i64;
        let created_by = "user-fixed-0001".to_string();
        let updated_at = created_at;
        let updated_by = created_by.clone();
        let deleted_at = None;
        let last_event_id = None;

        let state = TimeEntryState::Registered {
            time_entry_id: time_entry_id.clone(),
            user_id: user_id.clone(),
            start_time,
            end_time,
            tags: tags.clone(),
            description: description.clone(),
            created_at,
            created_by: created_by.clone(),
            updated_at,
            updated_by: updated_by.clone(),
            deleted_at,
            last_event_id,
        };

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
                assert_eq!(te, time_entry_id);
                assert_eq!(u, user_id);
                assert_eq!(s, start_time);
                assert_eq!(e, end_time);
                assert_eq!(t, tags);
                assert_eq!(d, description);
                assert_eq!(ca, created_at);
                assert_eq!(cb, created_by);
                assert_eq!(ua, updated_at);
                assert_eq!(ub, updated_by);
                assert_eq!(del, None);
                assert_eq!(le, None);
            }
            _ => panic!("expected Registered state"),
        }
    }
}
