use arts_n_crafts::core::base_payload::BasePayload;
use arts_n_crafts::domain::domain_event::DomainEvent;
use arts_n_crafts::infrastructure::event_bus::event_producer::{EventProducer, EventProducerError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::modules::time_entries::infrastructure::event_bus::PulsarEventBus;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PulsarProducerMessageProperties {
    aggregate_id: String,
    event_type: String,
    timestamp: i64,
    metadata: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PulsarProducerMessage {
    payload: String,
    key: Option<String>,
    properties: Option<PulsarProducerMessageProperties>,
    context: Option<String>,
    replication_clusters: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PulsarProducerBody {
    producer_name: &'static str,
    messages: Vec<PulsarProducerMessage>,
}

#[async_trait]
impl<TEventPayload> EventProducer<TEventPayload> for PulsarEventBus
where
    TEventPayload: BasePayload + 'static,
{
    async fn publish(&self, stream: String, event: DomainEvent<TEventPayload>) -> Result<(), EventProducerError> {
        let client = Client::new();

        let message_properties = PulsarProducerMessageProperties {
            aggregate_id: event.aggregate_id.clone(),
            event_type: event.event_type.clone(),
            timestamp: event.timestamp,
            metadata: serde_json::to_string(&event.metadata).unwrap(),
        };
        let message = PulsarProducerMessage {
            payload: serde_json::to_string(&event.payload).unwrap(),
            key: Some(event.aggregate_id.clone()),
            properties: Some(message_properties),
            context: Some(event.event_id.clone()),
            replication_clusters: None,
        };
        let body = PulsarProducerBody {
            producer_name: self.producer_name,
            messages: vec![message],
        };

        client
            .post(format!("{}/topics/persistent/{}/{}/{}", self.broker_url, self.tenant, self.namespace, stream))
            .json(&body)
            .send()
            .await
            .map_err(|_err| EventProducerError::PublishEventFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod pulsar_event_bus_tests {
    use super::*;
    use crate::modules::time_entries::domain::time_entry_event_payload::TimeEntryEventPayload;
    use crate::modules::time_entries::domain::time_entry_registered_event::create_time_entry_event;
    use chrono::{Duration, Utc};
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn time_entry_registered_event() -> DomainEvent<TimeEntryEventPayload> {
        let aggregate_id = Uuid::now_v7();
        let payload = TimeEntryEventPayload::TimeEntryRegistered {
            user_id: Uuid::now_v7().to_string(),
            start_time: (Utc::now() - Duration::hours(2)).timestamp_millis(),
            end_time: Utc::now().timestamp_millis(),
        };
        create_time_entry_event(aggregate_id, payload)
    }


    #[rstest]
    #[tokio::test]
    #[ignore]
    async fn pulsar_should_publish_the_event(mut time_entry_registered_event: DomainEvent<TimeEntryEventPayload>) {
        time_entry_registered_event.set_correlation_id(Uuid::now_v7().to_string());
        let stream = "time_entries".to_string();

        let bus = PulsarEventBus::new(
            "time_entries_producer",
            "http://localhost:8080",
            "public",
            "default"
        );
        let result = bus.publish(stream, time_entry_registered_event).await;
        assert!(result.is_ok());
    }
}
