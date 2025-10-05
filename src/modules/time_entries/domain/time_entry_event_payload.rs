use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, AsRefStr)]
pub enum TimeEntryEventPayload {
    TimeEntryRegistered {
        user_id: String,
        start_time: i64,
        end_time: i64,
    },
}

#[cfg(test)]
mod time_entry_event_payload_tests {
    use super::*;
    use chrono::Utc;
    use rstest::rstest;
    use uuid::Uuid;

    #[rstest]
    fn should_create_the_payload() {
        let user_id = Uuid::now_v7().to_string();
        let start_time = Utc::now().timestamp_millis();
        let end_time = Utc::now().timestamp_millis();

        let payload = TimeEntryEventPayload::TimeEntryRegistered {
            user_id: user_id.clone(),
            start_time: start_time.clone(),
            end_time: end_time.clone(),
        };

        let TimeEntryEventPayload::TimeEntryRegistered { user_id: payload_user_id, start_time: payload_start_time, end_time: payload_end_time } = &payload;
        assert_eq!(payload_user_id, &user_id);
        assert_eq!(payload_start_time, &start_time);
        assert_eq!(payload_end_time, &end_time);
    }
}
