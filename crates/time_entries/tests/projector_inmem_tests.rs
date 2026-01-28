// Tests for the projector mapping and runner using in memory repositories.
//
// Responsibilities when you add code
// - Feed a registration event and assert an upserted read model row exists.
// - Feed the same event again and assert idempotency via last processed event identifier.
