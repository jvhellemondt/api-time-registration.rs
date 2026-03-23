#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeEntryState {
    None,
    Draft {
        time_entry_id: String,
        user_id: String,
        started_at: Option<i64>,
        ended_at: Option<i64>,
        tag_ids: Vec<String>,
        created_at: i64,
        created_by: String,
    },
    Registered {
        time_entry_id: String,
        user_id: String,
        started_at: i64,
        ended_at: i64,
        tag_ids: Vec<String>,
        created_at: i64,
        created_by: String,
    },
}

#[cfg(test)]
mod time_entry_state_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn it_should_create_the_draft_state() {
        let state = TimeEntryState::Draft {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            started_at: None,
            ended_at: None,
            tag_ids: vec![],
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
        };
        match state {
            TimeEntryState::Draft {
                time_entry_id,
                user_id,
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(time_entry_id, "te-fixed-0001");
                assert_eq!(user_id, "user-fixed-0001");
                assert_eq!(started_at, None);
                assert_eq!(ended_at, None);
            }
            _ => panic!("expected Draft state"),
        }
    }

    #[rstest]
    fn it_should_create_the_registered_state() {
        let state = TimeEntryState::Registered {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            started_at: 1_700_000_000_000i64,
            ended_at: 1_700_000_360_000i64,
            tag_ids: vec![],
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
        };
        match state {
            TimeEntryState::Registered {
                time_entry_id,
                user_id,
                started_at,
                ended_at,
                ..
            } => {
                assert_eq!(time_entry_id, "te-fixed-0001");
                assert_eq!(user_id, "user-fixed-0001");
                assert_eq!(started_at, 1_700_000_000_000i64);
                assert_eq!(ended_at, 1_700_000_360_000i64);
            }
            _ => panic!("expected Registered state"),
        }
    }
}
