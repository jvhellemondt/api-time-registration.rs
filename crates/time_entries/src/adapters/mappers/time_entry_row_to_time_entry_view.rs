use crate::application::query_handlers::time_entries_queries::TimeEntryView;
use crate::core::time_entry::projector::model::TimeEntryRow;

impl From<TimeEntryRow> for TimeEntryView {
    fn from(row: TimeEntryRow) -> Self {
        Self {
            time_entry_id: row.time_entry_id,
            user_id: row.user_id,
            start_time: row.start_time,
            end_time: row.end_time,
            tags: row.tags,
            description: row.description,
            created_at: row.created_at,
            created_by: row.created_by,
            updated_at: row.updated_at,
            updated_by: row.updated_by,
            deleted_at: row.deleted_at,
        }
    }
}
