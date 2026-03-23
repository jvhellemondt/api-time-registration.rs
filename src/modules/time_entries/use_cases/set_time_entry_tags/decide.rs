use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::events::v1::time_entry_initiated::TimeEntryInitiatedV1;
use crate::modules::time_entries::core::events::v1::time_entry_tags_set::TimeEntryTagsSetV1;
use crate::modules::time_entries::core::intents::TimeEntryIntent;
use crate::modules::time_entries::core::state::TimeEntryState;
use crate::modules::time_entries::use_cases::set_time_entry_tags::command::SetTimeEntryTags;
use crate::modules::time_entries::use_cases::set_time_entry_tags::decision::Decision;

pub fn decide_set_time_entry_tags(state: &TimeEntryState, command: SetTimeEntryTags) -> Decision {
    let tags_set_event = TimeEntryEvent::TimeEntryTagsSetV1(TimeEntryTagsSetV1 {
        time_entry_id: command.time_entry_id.clone(),
        tag_ids: command.tag_ids,
        updated_at: command.updated_at,
        updated_by: command.updated_by.clone(),
    });

    let notify = TimeEntryIntent::NotifyUser {
        time_entry_id: command.time_entry_id.clone(),
        occurred_at: command.updated_at,
    };

    match state {
        TimeEntryState::None => {
            let initiated = TimeEntryEvent::TimeEntryInitiatedV1(TimeEntryInitiatedV1 {
                time_entry_id: command.time_entry_id,
                user_id: command.user_id,
                created_at: command.updated_at,
                created_by: command.updated_by,
            });
            Decision::Accepted {
                events: vec![initiated, tags_set_event],
                intents: vec![notify],
            }
        }
        TimeEntryState::Draft { .. } => Decision::Accepted {
            events: vec![tags_set_event],
            intents: vec![notify],
        },
        TimeEntryState::Registered { .. } => Decision::Accepted {
            events: vec![tags_set_event],
            intents: vec![notify],
        },
    }
}

#[cfg(test)]
mod decide_set_time_entry_tags_tests {
    use super::*;
    use crate::modules::time_entries::core::intents::TimeEntryIntent;
    use crate::tests::fixtures::commands::set_time_entry_tags::SetTimeEntryTagsBuilder;
    use rstest::{fixture, rstest};

    #[fixture]
    fn command() -> SetTimeEntryTags {
        SetTimeEntryTagsBuilder::new().build()
    }

    #[rstest]
    fn it_should_emit_initiated_and_tags_set_when_none(command: SetTimeEntryTags) {
        let decision = decide_set_time_entry_tags(&TimeEntryState::None, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 2);
                assert!(matches!(
                    &events[0],
                    TimeEntryEvent::TimeEntryInitiatedV1(_)
                ));
                assert!(matches!(&events[1], TimeEntryEvent::TimeEntryTagsSetV1(_)));
                assert_eq!(intents.len(), 1);
                assert!(matches!(&intents[0], TimeEntryIntent::NotifyUser { .. }));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_emit_tags_set_when_draft(command: SetTimeEntryTags) {
        let state = TimeEntryState::Draft {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: None,
            ended_at: None,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_time_entry_tags(&state, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 1);
                assert!(matches!(&events[0], TimeEntryEvent::TimeEntryTagsSetV1(_)));
                assert_eq!(intents.len(), 1);
                assert!(matches!(&intents[0], TimeEntryIntent::NotifyUser { .. }));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_emit_tags_set_when_registered(command: SetTimeEntryTags) {
        let state = TimeEntryState::Registered {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: 1_000,
            ended_at: 2_000,
            tag_ids: vec![],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_time_entry_tags(&state, command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 1);
                assert!(matches!(&events[0], TimeEntryEvent::TimeEntryTagsSetV1(_)));
                assert_eq!(intents.len(), 1);
                assert!(matches!(&intents[0], TimeEntryIntent::NotifyUser { .. }));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_replace_existing_tags_when_draft() {
        let command = SetTimeEntryTagsBuilder::new()
            .tag_ids(vec!["new-tag".to_string()])
            .build();
        let state = TimeEntryState::Draft {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: None,
            ended_at: None,
            tag_ids: vec!["old-tag".to_string()],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_time_entry_tags(&state, command);
        match decision {
            Decision::Accepted { events, .. } => {
                assert!(
                    matches!(&events[0], TimeEntryEvent::TimeEntryTagsSetV1(e) if e.tag_ids == vec!["new-tag".to_string()])
                );
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_allow_clearing_tags() {
        let command = SetTimeEntryTagsBuilder::new().tag_ids(vec![]).build();
        let state = TimeEntryState::Draft {
            time_entry_id: command.time_entry_id.clone(),
            user_id: command.user_id.clone(),
            started_at: None,
            ended_at: None,
            tag_ids: vec!["tag-1".to_string()],
            created_at: 0,
            created_by: command.updated_by.clone(),
        };
        let decision = decide_set_time_entry_tags(&state, command);
        match decision {
            Decision::Accepted { events, .. } => {
                assert!(
                    matches!(&events[0], TimeEntryEvent::TimeEntryTagsSetV1(e) if e.tag_ids.is_empty())
                );
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }
}
