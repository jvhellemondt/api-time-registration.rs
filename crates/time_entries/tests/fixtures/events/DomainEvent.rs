#[derive(Debug, Clone)]
pub struct DomainEvent<'a> {
    pub event_type: &'a str,
}
