// Event payload: TimeEntryRegisteredV1.
//
// Purpose
// - Record the business fact that a time entry was registered with the minimal fields.
//
// Responsibilities
// - Carry only identifiers and snapshot values needed by the domain today.
//
// Inputs and outputs
// - Inputs: values from the command validated by the decider.
// - Outputs: fed into evolve to produce the first registered state and into projectors.
//
// Versioning and evolution
// - Prefer adding fields. For breaking changes, create TimeEntryRegisteredV2 in a new file and add a new variant.

