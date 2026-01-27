// Ports define what the core needs from the outside world, without implementing it.
//
// Purpose
// - Describe abstract input and output capabilities as traits (for example: EventStore, DomainOutbox).
//
// Responsibilities
// - Keep the core independent from any database or broker by coding against traits.
//
// Boundaries
// - No concrete input or output here. Adapters implement these traits in the adapters layer.
//
// Testing guidance
// - Provide in memory implementations for tests and local development.

