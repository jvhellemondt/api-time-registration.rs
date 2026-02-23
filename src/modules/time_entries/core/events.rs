pub mod v1 {
    pub mod time_entry_registered;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TimeEntryEvent {
    TimeEntryRegisteredV1(v1::time_entry_registered::TimeEntryRegisteredV1),
}
