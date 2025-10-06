mod pulsar_event_producer;

#[derive(Debug, Clone)]
struct PulsarEventBus {
    producer_name: &'static str,
    broker_url: &'static str,
    tenant: &'static str,
    namespace: &'static str,
}

impl PulsarEventBus {
    pub fn new(
        producer_name: &'static str,
        broker_url: &'static str,
        tenant: &'static str,
        namespace: &'static str,
    ) -> PulsarEventBus {
        PulsarEventBus {
            producer_name,
            broker_url,
            tenant,
            namespace,
        }
    }
}
