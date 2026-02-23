# time-registration repository

Purpose
- This repository contains the backend implementation for the time registration bounded context.
- It is structured as a Rust workspace. This scaffold creates the time entries crate.

Structure
- crates/time_entries: the time entries bounded context.
- Inside the crate, code is split into:
  - core (pure domain logic)
  - application (imperative orchestration)
  - adapters (input and output implementations)
  - shell (developer runners and composition)

Guiding principles
- Keep the core pure and free of input or output.
- Put orchestration and transactions in application.
- Place all input and output in adapters and shell.
- Write unit tests in core, contract tests for adapters, and end-to-end tests across layers.

Evolution rules
- Prefer additive changes.
- Version events for breaking changes.

---

### 🛠️ Development Tools

This project is a Rust application that uses several `cargo` subcommands to streamline the development process. We've
integrated powerful tools like **`cargo-llvm-cov`** and **`cargo-nextest`** for comprehensive code coverage and fast,
efficient test execution. The use of **`cargo-run-script`** helps automate build tasks, while **`cargo-audit`** ensures
our dependencies are secure.

To get started with this project, you'll need a few essential `cargo` subcommands. Install them by running the following
commands in your terminal. We recommend that all contributors install these tools to maintain a consistent and secure
development environment.

#### For generating code coverage reports
```sh
cargo install cargo-llvm-cov
```

#### For a fast and powerful test runner
```sh
cargo install cargo-nextest
```

#### To run scripts defined in your Cargo.toml
```sh
cargo install cargo-run-script
```

#### To check for security vulnerabilities in project dependencies
```sh
cargo install cargo-audit
```

#### To run the various commands:
`cd` into the crate in which the `cargo.toml` that contains the "package.metadata.scripts"-block and run:

```sh
cargo run-script <<cargo.toml command>>
```

Examples:
```sh
cargo run-script test
```
