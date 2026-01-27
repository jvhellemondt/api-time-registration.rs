// Registration command handler orchestrates the write flow.
//
// Responsibilities
// - Load past events from the event store and fold them into state.
// - Call the decider with the command and current time.
// - Append new events with optimistic concurrency.
// - Enqueue domain events into the domain outbox for publishing.

