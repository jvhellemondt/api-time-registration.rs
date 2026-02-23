use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DomainEvent<'a> {
    pub name: &'a str,
}
