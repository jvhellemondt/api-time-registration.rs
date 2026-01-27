# core/time_entry/decider folder

Purpose
- Holds pure decision logic for each command intent. Each intent gets its own subfolder or file.

What belongs here
- A subfolder per intent containing:
  - command data type
  - decide function that validates and produces events

Boundaries
- No input or output. No database or broker logic.
- Accept current time and other external values as function parameters.

