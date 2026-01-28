// Shared test fixture for the RegisterTimeEntry command.
// This file is included into the crate only during tests via `include!`
// in `src/lib.rs`, exposing it under `time_entries::test_fixtures`.

use std::fs;
use crate::core::time_entry::decider::register::command::RegisterTimeEntry;
use serde::Deserialize;

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

impl RegisterTimeEntryBuilder {
    pub fn new() -> Self {
        let json_str = fs::read_to_string("./tests/fixtures/commands/json/register_time_entry.json").unwrap();
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

/// Build a canonical, valid registration command for tests.
/// All timestamps use epoch milliseconds consistently.
pub fn make_register_time_entry_command() -> RegisterTimeEntry {
    RegisterTimeEntryBuilder::new().build()
}
