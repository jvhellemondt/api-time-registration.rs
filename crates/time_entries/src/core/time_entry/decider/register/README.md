# decider/register folder

Purpose
- Defines the command and decision rules for registering a new time entry.

What belongs here
- command.rs: the command data structure for registration.
- decide.rs: the pure function that validates the command and emits an event.
- The handler for this command lives in application/command_handlers.

Boundaries
- Pure logic only. No input or output.

