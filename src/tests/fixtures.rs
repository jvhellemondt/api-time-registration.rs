pub mod events {
    pub mod domain_event;
    pub mod time_entry_end_set_v1;
    pub mod time_entry_initiated_v1;
    pub mod time_entry_registered_v1;
    pub mod time_entry_start_set_v1;
}
pub mod commands {
    pub mod set_ended_at;
    pub mod set_started_at;
}
pub mod tags;
