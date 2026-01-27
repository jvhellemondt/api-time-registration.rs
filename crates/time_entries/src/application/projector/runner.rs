// Projector runner consumes a stream of events, translates them into mutations,
// persists them using a repository, and advances the watermark.
//
// Purpose
// - Guarantee idempotent application of events and safe recovery on failure.

