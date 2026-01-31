// Translate a domain event into read model mutations.
//
// Purpose
// - Build an upsert for registration and minimal patches for future change events.
//
// Responsibilities
// - Calculate last_event_id as a stable identifier like "stream_id:version".
// - Return a list of mutations to be persisted by the application runner.

use crate::core::time_entry::{event::TimeEntryEvent, projector::model::TimeEntryRow};

pub enum Mutation {
    Upsert(TimeEntryRow),
}

pub fn apply(stream_id: &str, version: i64, event: &TimeEntryEvent) -> Vec<Mutation> {
    let stream_key = format!("{stream_id}:{version}");
    match event {
        TimeEntryEvent::TimeEntryRegisteredV1(details) => vec![Mutation::Upsert(TimeEntryRow {
            time_entry_id: details.time_entry_id.clone(),
            user_id: details.user_id.clone(),
            start_time: details.start_time,
            end_time: details.end_time,
            tags: details.tags.clone(),
            description: details.description.clone(),
            created_at: details.created_at,
            created_by: details.created_by.clone(),
            updated_at: details.created_at,
            updated_by: details.created_by.clone(),
            deleted_at: None,
            last_event_id: Some(stream_key),
        })],
    }
}

#[cfg(test)]
mod time_entry_projector_apply_tests {
    use super::*;
    use crate::test_support::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::rstest;

    #[rstest]
    fn it_should_apply_the_event() {
        let stream_id = "time-entries-0001";
        let event = make_time_entry_registered_v1_event();
        let mutations = apply(stream_id, 1, &TimeEntryEvent::TimeEntryRegisteredV1(event));
        assert_eq!(mutations.len(), 1);
        assert!(
            matches!(&mutations[0], Mutation::Upsert(TimeEntryRow { .. })),
            "expected first mutation to be Upsert(..) with a TimeEntryRow"
        );
    }
}
