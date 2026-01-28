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

/// Build a canonical, valid registration command for tests.
/// All timestamps use epoch milliseconds consistently.
pub fn make_register_time_entry_command() -> RegisterTimeEntry {
    let json_str = fs::read_to_string("./tests/fixtures/commands/json/register_time_entry.json").unwrap();
    let dto: RegisterTimeEntryDto = serde_json::from_str(&json_str).unwrap();

    RegisterTimeEntry {
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

pub fn make_register_time_entry_command_with(
    time_entry_id: Option<String>,
    user_id: Option<String>,
    start_time: Option<i64>,
    end_time: Option<i64>,
    tags: Option<Vec<String>>,
    description: Option<String>,
    created_at: Option<i64>,
    created_by: Option<String>,
) -> RegisterTimeEntry {
    let base = make_register_time_entry_command();
    RegisterTimeEntry {
        time_entry_id: time_entry_id.unwrap_or(base.time_entry_id),
        user_id: user_id.unwrap_or(base.user_id),
        start_time: start_time.unwrap_or(base.start_time),
        end_time: end_time.unwrap_or(base.end_time),
        tags: tags.unwrap_or(base.tags),
        description: description.unwrap_or(base.description),
        created_at: created_at.unwrap_or(base.created_at),
        created_by: created_by.unwrap_or(base.created_by),
    }
}
