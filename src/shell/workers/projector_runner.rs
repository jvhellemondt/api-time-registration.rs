// Spawns each projector as an independent async task.
//
// The shell calls `spawn` once per projector at startup, passing the projector
// its broadcast receiver. Each projector runs its own loop independently.

use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::ListTimeEntriesState;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projector::ListTimeEntriesProjector;
use crate::shared::infrastructure::event_store::StoredEvent;
use crate::shared::infrastructure::projection_store::ProjectionStore;
use tokio::sync::broadcast;

pub fn spawn<TStore>(
    projector: ListTimeEntriesProjector<TStore>,
    receiver: broadcast::Receiver<StoredEvent<TimeEntryEvent>>,
) where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    tokio::spawn(projector.run(receiver));
}
