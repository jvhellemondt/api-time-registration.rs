FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app

# Cache dependencies by building with a stub binary first
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/shell && echo 'fn main() {}' > src/shell/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl --bin time_entries
RUN rm -rf src

# Build the real binary
COPY src ./src
RUN touch src/shell/main.rs && cargo build --release --target x86_64-unknown-linux-musl --bin time_entries

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/time_entries /usr/local/bin/time_entries
EXPOSE 8080
CMD ["time_entries"]
