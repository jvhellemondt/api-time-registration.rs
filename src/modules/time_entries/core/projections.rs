use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries::projection::{
    TimeEntryRow, TimeEntryStatus,
};

pub enum Mutation {
    Upsert(TimeEntryRow),
    SetStartedAt {
        time_entry_id: String,
        started_at: i64,
        updated_at: i64,
        updated_by: String,
        last_event_id: String,
    },
    SetEndedAt {
        time_entry_id: String,
        ended_at: i64,
        updated_at: i64,
        updated_by: String,
        last_event_id: String,
    },
    SetRegistered {
        time_entry_id: String,
        last_event_id: String,
    },
    SetDeleted {
        time_entry_id: String,
        deleted_at: i64,
        last_event_id: String,
    },
    SetTags {
        time_entry_id: String,
        tag_ids: Vec<String>,
        updated_at: i64,
        updated_by: String,
        last_event_id: String,
    },
}

pub fn apply(stream_id: &str, version: i64, event: &TimeEntryEvent) -> Vec<Mutation> {
    let last_event_id = format!("{stream_id}:{version}");
    match event {
        TimeEntryEvent::TimeEntryInitiatedV1(e) => vec![Mutation::Upsert(TimeEntryRow {
            time_entry_id: e.time_entry_id.clone(),
            user_id: e.user_id.clone(),
            started_at: None,
            ended_at: None,
            tag_ids: vec![],
            status: TimeEntryStatus::Draft,
            created_at: e.created_at,
            created_by: e.created_by.clone(),
            updated_at: e.created_at,
            updated_by: e.created_by.clone(),
            deleted_at: None,
            last_event_id: Some(last_event_id),
        })],
        TimeEntryEvent::TimeEntryStartSetV1(e) => vec![Mutation::SetStartedAt {
            time_entry_id: e.time_entry_id.clone(),
            started_at: e.started_at,
            updated_at: e.updated_at,
            updated_by: e.updated_by.clone(),
            last_event_id,
        }],
        TimeEntryEvent::TimeEntryEndSetV1(e) => vec![Mutation::SetEndedAt {
            time_entry_id: e.time_entry_id.clone(),
            ended_at: e.ended_at,
            updated_at: e.updated_at,
            updated_by: e.updated_by.clone(),
            last_event_id,
        }],
        TimeEntryEvent::TimeEntryRegisteredV1(e) => vec![Mutation::SetRegistered {
            time_entry_id: e.time_entry_id.clone(),
            last_event_id,
        }],
        TimeEntryEvent::TimeEntryDeletedV1(e) => vec![Mutation::SetDeleted {
            time_entry_id: e.time_entry_id.clone(),
            deleted_at: e.deleted_at,
            last_event_id,
        }],
        TimeEntryEvent::TimeEntryTagsSetV1(e) => vec![Mutation::SetTags {
            time_entry_id: e.time_entry_id.clone(),
            tag_ids: e.tag_ids.clone(),
            updated_at: e.updated_at,
            updated_by: e.updated_by.clone(),
            last_event_id,
        }],
    }
}

#[cfg(test)]
mod time_entry_projector_apply_tests {
    use super::*;
    use crate::modules::time_entries::core::events::v1::time_entry_deleted::TimeEntryDeletedV1;
    use crate::modules::time_entries::core::events::v1::time_entry_end_set::TimeEntryEndSetV1;
    use crate::modules::time_entries::core::events::v1::time_entry_initiated::TimeEntryInitiatedV1;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::modules::time_entries::core::events::v1::time_entry_start_set::TimeEntryStartSetV1;
    use crate::modules::time_entries::core::events::v1::time_entry_tags_set::TimeEntryTagsSetV1;
    use rstest::rstest;

    const STREAM_ID: &str = "TimeEntry-te-0001";

    #[rstest]
    fn it_should_apply_initiated_event() {
        let event = TimeEntryEvent::TimeEntryInitiatedV1(TimeEntryInitiatedV1 {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            created_at: 1_000,
            created_by: "user-0001".to_string(),
        });
        let mutations = apply(STREAM_ID, 1, &event);
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::Upsert(_)));
    }

    #[rstest]
    fn it_should_apply_start_set_event() {
        let event = TimeEntryEvent::TimeEntryStartSetV1(TimeEntryStartSetV1 {
            time_entry_id: "te-0001".to_string(),
            started_at: 500,
            updated_at: 1_000,
            updated_by: "user-0001".to_string(),
        });
        let mutations = apply(STREAM_ID, 2, &event);
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetStartedAt { .. }));
    }

    #[rstest]
    fn it_should_apply_end_set_event() {
        let event = TimeEntryEvent::TimeEntryEndSetV1(TimeEntryEndSetV1 {
            time_entry_id: "te-0001".to_string(),
            ended_at: 800,
            updated_at: 1_000,
            updated_by: "user-0001".to_string(),
        });
        let mutations = apply(STREAM_ID, 3, &event);
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetEndedAt { .. }));
    }

    #[rstest]
    fn it_should_apply_registered_event() {
        let event = TimeEntryEvent::TimeEntryRegisteredV1(TimeEntryRegisteredV1 {
            time_entry_id: "te-0001".to_string(),
            occurred_at: 1_000,
        });
        let mutations = apply(STREAM_ID, 4, &event);
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetRegistered { .. }));
    }

    #[rstest]
    fn it_should_apply_deleted_event() {
        let event = TimeEntryEvent::TimeEntryDeletedV1(TimeEntryDeletedV1 {
            time_entry_id: "te-0001".to_string(),
            deleted_at: 2_000,
            deleted_by: "user-0001".to_string(),
        });
        let mutations = apply(STREAM_ID, 5, &event);
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetDeleted { .. }));
    }

    #[rstest]
    fn it_should_apply_tags_set_event() {
        let event = TimeEntryEvent::TimeEntryTagsSetV1(TimeEntryTagsSetV1 {
            time_entry_id: "te-0001".to_string(),
            tag_ids: vec!["tag-1".to_string(), "tag-2".to_string()],
            updated_at: 1_000,
            updated_by: "user-0001".to_string(),
        });
        let mutations = apply(STREAM_ID, 6, &event);
        assert_eq!(mutations.len(), 1);
        assert!(matches!(&mutations[0], Mutation::SetTags { .. }));
    }
}
