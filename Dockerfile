FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y libsqlite3-dev pkg-config
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libsqlite3-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/GolemianDiscordBot .
CMD ["./GolemianDiscordBot"]
