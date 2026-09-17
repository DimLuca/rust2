FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/araquaridesk-server /usr/local/bin/araquaridesk-server
COPY --from=builder /app/target/release/araquaridesk-admin /usr/local/bin/araquaridesk-admin
USER 65532:65532
EXPOSE 8787
ENTRYPOINT ["araquaridesk-server"]
