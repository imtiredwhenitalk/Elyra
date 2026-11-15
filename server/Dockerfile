FROM rust:1.85-bookworm AS builder
WORKDIR /workspace

# The repository is a Cargo workspace, so every member manifest must be present
# for dependency resolution even though only the server is compiled here.
# .dockerignore keeps node_modules, build output, and local databases out.
COPY . .
RUN cargo build --release -p elyra-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/target/release/elyra-server /usr/local/bin/elyra-server
USER 10001:10001
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/elyra-server"]
