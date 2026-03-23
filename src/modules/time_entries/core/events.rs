pub mod v1 {
    pub mod time_entry_deleted;
    pub mod time_entry_end_set;
    pub mod time_entry_initiated;
    pub mod time_entry_registered;
    pub mod time_entry_start_set;
    pub mod time_entry_tags_set;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TimeEntryEvent {
    TimeEntryInitiatedV1(v1::time_entry_initiated::TimeEntryInitiatedV1),
    TimeEntryStartSetV1(v1::time_entry_start_set::TimeEntryStartSetV1),
    TimeEntryEndSetV1(v1::time_entry_end_set::TimeEntryEndSetV1),
    TimeEntryRegisteredV1(v1::time_entry_registered::TimeEntryRegisteredV1),
    TimeEntryDeletedV1(v1::time_entry_deleted::TimeEntryDeletedV1),
    TimeEntryTagsSetV1(v1::time_entry_tags_set::TimeEntryTagsSetV1),
}
