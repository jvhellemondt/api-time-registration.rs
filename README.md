# api-time-registration.rs

---

### Using Pulsar for eventing

Add super user:
```shell
CSRF_TOKEN=$(curl http://localhost:7750/pulsar-manager/csrf-token)
curl \
    -H "X-XSRF-TOKEN: $CSRF_TOKEN" \
    -H "Cookie: XSRF-TOKEN=$CSRF_TOKEN;" \
    -H 'Content-Type: application/json' \
    -X PUT http://localhost:7750/pulsar-manager/users/superuser \
    -d '{"name": "admin", "password": "pulsar", "description": "test", "email": "username@test.org"}'
```

Environment Name: dev
Service URL: http://pulsar:8080
Bookie URL: http://pulsar:8080
[source](https://jpinjpblog.wordpress.com/2020/12/10/pulsar-with-manager-and-dashboard-on-docker-compose/)


---

This project is a Rust application that uses several `cargo` subcommands to streamline the development process. We've
integrated powerful tools like **`cargo-llvm-cov`** and **`cargo-nextest`** for comprehensive code coverage and fast,
efficient test execution. The use of **`cargo-run-script`** helps automate build tasks, while **`cargo-audit`** ensures
our dependencies are secure.

### 🛠️ Development Tools

To get started with this project, you'll need a few essential `cargo` subcommands. Install them by running the following
commands in your terminal. We recommend that all contributors install these tools to maintain a consistent and secure
development environment.

```sh
# For generating code coverage reports
cargo install cargo-llvm-cov
```

```sh
# For a fast and powerful test runner
cargo install cargo-nextest
```

```sh
# To run scripts defined in your Cargo.toml
cargo install cargo-run-script
```

```sh
# To check for security vulnerabilities in project dependencies
cargo install cargo-audit
```
