#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_registered_command_tests {
    use super::*;
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use rstest::{fixture, rstest};

    #[fixture]
    fn register_command() -> RegisterTimeEntry {
        RegisterTimeEntryBuilder::new().build()
    }

    #[rstest]
    fn it_should_create_the_command(register_command: RegisterTimeEntry) {
        assert_eq!(register_command.time_entry_id, "te-fixed-0001");
        assert_eq!(register_command.user_id, "user-fixed-0001");
        assert_eq!(register_command.tags, vec!["Work".to_string()]);
    }
}
