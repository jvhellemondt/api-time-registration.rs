// In memory projection repository, watermark repository, and query handler.
//
// Purpose
// - Exercise projectors and queries without a database.
//
// Responsibilities
// - Store read model rows in a map keyed by identifiers.
// - Track last processed event per projector.
// - Implement query handler traits for reads.

