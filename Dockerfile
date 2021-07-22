FROM rust:latest as base

WORKDIR /passage

# Install dependencies
COPY Cargo.toml Cargo.lock .
RUN mkdir src \
    && touch src/lib.rs \
    && cargo fetch

COPY . .
ENV RUST_LOG=trace
EXPOSE 12345

FROM base as development
ENTRYPOINT cargo run --bin passage-server

FROM base as builder
RUN cargo build --release --offline

FROM ubuntu:latest
COPY --from=builder /passage/target/release/passage-server .
ENTRYPOINT ./passage-server
