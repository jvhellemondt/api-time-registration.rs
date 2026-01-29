// Shared test fixture for the RegisterTimeEntry command.
// This file is included in the crate only during tests via `include!`
// in `src/lib.rs`, exposing it under `time_entries::test_fixtures`.

use crate::core::time_entry::decider::register::command::RegisterTimeEntry;
use serde::Deserialize;
use std::fs;

// JSON -> DTO (transport shape)
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterTimeEntryDto {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
}

pub struct RegisterTimeEntryBuilder {
    inner: RegisterTimeEntry,
}

impl Default for RegisterTimeEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl RegisterTimeEntryBuilder {
    pub fn new() -> Self {
        let json_str = fs::read_to_string("./src/test_support/fixtures/commands/json/register_time_entry.json").unwrap();
        let dto: RegisterTimeEntryDto = serde_json::from_str(&json_str).unwrap();

        Self {
            inner: RegisterTimeEntry {
                time_entry_id: dto.time_entry_id,
                user_id: dto.user_id,
                start_time: dto.start_time,
                end_time: dto.end_time,
                tags: dto.tags,
                description: dto.description,
                created_by: "user-fixed-0001".to_string(),
                created_at: 1700000000000,
            }
        }
    }

    pub fn time_entry_id(mut self, v: impl Into<String>) -> Self {
        self.inner.time_entry_id = v.into();
        self
    }

    pub fn user_id(mut self, v: impl Into<String>) -> Self {
        self.inner.user_id = v.into();
        self
    }

    pub fn start_time(mut self, v: i64) -> Self {
        self.inner.start_time = v;
        self
    }

    pub fn end_time(mut self, v: i64) -> Self {
        self.inner.end_time = v;
        self
    }

    pub fn tags(mut self, v: Vec<String>) -> Self {
        self.inner.tags = v;
        self
    }

    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.inner.description = v.into();
        self
    }

    pub fn created_at(mut self, v: i64) -> Self {
        self.inner.created_at = v;
        self
    }

    pub fn created_by(mut self, v: impl Into<String>) -> Self {
        self.inner.created_by = v.into();
        self
    }

    pub fn build(self) -> RegisterTimeEntry {
        self.inner
    }
}

#[cfg(test)]
mod time_entry_register_time_entry_builder_tests {
    use rstest::rstest;
    use super::*;

    #[rstest]
    fn default_delegates_to_new_and_parses_json() {
        let dto = RegisterTimeEntryDto {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            start_time: 1700000000000,
            end_time: 1700000360000,
            tags: vec!["Work".to_string()],
            description: "This is a test".to_string(),
        };

        let built = RegisterTimeEntryBuilder::default().build();
        assert_eq!(built.time_entry_id, dto.time_entry_id);
        assert_eq!(built.user_id, dto.user_id);
        assert_eq!(built.start_time, dto.start_time);
        assert_eq!(built.end_time, dto.end_time);
        assert_eq!(built.tags, dto.tags);
        assert_eq!(built.description, dto.description);
        assert_eq!(built.created_by, "user-fixed-0001");
        assert_eq!(built.created_at, 1_700_000_000_000i64);
    }

    #[rstest]
    fn setters_override_all_fields_and_build_returns_inner() {
        let custom = RegisterTimeEntryBuilder::new()
            .time_entry_id("tid-123")
            .user_id("uid-456")
            .start_time(1111)
            .end_time(2222)
            .tags(vec!["a".into(), "b".into()])
            .description("desc")
            .created_by("tester")
            .created_at(3333)
            .build();

        assert_eq!(custom.time_entry_id, "tid-123");
        assert_eq!(custom.user_id, "uid-456");
        assert_eq!(custom.start_time, 1111);
        assert_eq!(custom.end_time, 2222);
        assert_eq!(custom.tags, vec!["a", "b"]);
        assert_eq!(custom.description, "desc");
        assert_eq!(custom.created_by, "tester");
        assert_eq!(custom.created_at, 3333);
    }
}
