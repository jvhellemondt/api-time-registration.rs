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
        let state = TimeEntryState::Registered {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            start_time: 1_700_000_000_000i64,
            end_time: 1_700_000_360_000i64,
            tags: vec!["Work".to_string()],
            description: "This is a test".to_string(),
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
            updated_at: 1_700_000_000_000i64,
            updated_by: "user-fixed-0001".to_string(),
            deleted_at: None,
            last_event_id: None,
        };
        match state {
            TimeEntryState::Registered {
                time_entry_id,
                user_id,
                tags,
                ..
            } => {
                assert_eq!(time_entry_id, "te-fixed-0001");
                assert_eq!(user_id, "user-fixed-0001");
                assert_eq!(tags, vec!["Work".to_string()]);
            }
            _ => panic!("expected Registered state"),
        }
    }
}
