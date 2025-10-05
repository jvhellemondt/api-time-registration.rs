use crate::modules::time_entries::domain::time_entry_event_payload::TimeEntryEventPayload;
use arts_n_crafts::domain::domain_event::DomainEvent;
use uuid::Uuid;

pub fn create_time_entry_event(
    aggregate_id: Uuid,
    payload: TimeEntryEventPayload,
) -> DomainEvent<TimeEntryEventPayload> {
    DomainEvent::create(aggregate_id.to_string(), payload)
}

#[cfg(test)]
mod create_time_entry_event_tests {
    use super::*;
    use chrono::Utc;
    use rstest::rstest;
    use uuid::Uuid;

    #[rstest]
    fn should_create_the_event() {
        let aggregate_id = Uuid::now_v7();
        let user_id = Uuid::now_v7().to_string();
        let start_time = Utc::now().timestamp_millis();
        let end_time = Utc::now().timestamp_millis();

        let payload = TimeEntryEventPayload::TimeEntryRegistered {
            user_id: user_id.clone(),
            start_time: start_time.clone(),
            end_time: end_time.clone(),
        };
        let event = create_time_entry_event(aggregate_id.clone(), payload.clone());
        let TimeEntryEventPayload::TimeEntryRegistered {
            user_id: event_payload_user_id,
            start_time: event_payload_start_time,
            end_time: event_payload_end_time
        } = &event.payload;

        assert_eq!(event.aggregate_id, aggregate_id.to_string());
        assert_eq!(event_payload_user_id, &user_id);
        assert_eq!(event_payload_start_time, &start_time);
        assert_eq!(event_payload_end_time, &end_time);

    }
}
