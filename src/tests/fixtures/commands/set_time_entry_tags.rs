use crate::modules::time_entries::use_cases::set_time_entry_tags::command::SetTimeEntryTags;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct SetTimeEntryTagsDto {
    pub time_entry_id: String,
    pub user_id: String,
    pub tag_ids: Vec<String>,
}

pub struct SetTimeEntryTagsBuilder {
    inner: SetTimeEntryTags,
}

impl Default for SetTimeEntryTagsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl SetTimeEntryTagsBuilder {
    pub fn new() -> Self {
        let json_str =
            fs::read_to_string("./src/tests/fixtures/commands/json/set_time_entry_tags.json")
                .unwrap();
        let dto: SetTimeEntryTagsDto = serde_json::from_str(&json_str).unwrap();

        Self {
            inner: SetTimeEntryTags {
                time_entry_id: dto.time_entry_id,
                user_id: dto.user_id,
                tag_ids: dto.tag_ids,
                updated_at: 1700000000000,
                updated_by: "user-fixed-0001".to_string(),
            },
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

    pub fn tag_ids(mut self, v: Vec<String>) -> Self {
        self.inner.tag_ids = v;
        self
    }

    pub fn updated_at(mut self, v: i64) -> Self {
        self.inner.updated_at = v;
        self
    }

    pub fn updated_by(mut self, v: impl Into<String>) -> Self {
        self.inner.updated_by = v.into();
        self
    }

    pub fn build(self) -> SetTimeEntryTags {
        self.inner
    }
}

#[cfg(test)]
mod set_time_entry_tags_builder_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn default_delegates_to_new_and_parses_json() {
        let built = SetTimeEntryTagsBuilder::default().build();
        assert_eq!(built.time_entry_id, "te-fixed-0001");
        assert_eq!(built.user_id, "user-fixed-0001");
        assert_eq!(built.tag_ids, vec!["tag-fixed-0001"]);
        assert_eq!(built.updated_at, 1700000000000);
        assert_eq!(built.updated_by, "user-fixed-0001");
    }

    #[rstest]
    fn setters_override_all_fields() {
        let custom = SetTimeEntryTagsBuilder::new()
            .time_entry_id("tid-123")
            .user_id("uid-456")
            .tag_ids(vec!["tag-a".to_string(), "tag-b".to_string()])
            .updated_at(2222)
            .updated_by("tester")
            .build();

        assert_eq!(custom.time_entry_id, "tid-123");
        assert_eq!(custom.user_id, "uid-456");
        assert_eq!(
            custom.tag_ids,
            vec!["tag-a".to_string(), "tag-b".to_string()]
        );
        assert_eq!(custom.updated_at, 2222);
        assert_eq!(custom.updated_by, "tester");
    }
}
