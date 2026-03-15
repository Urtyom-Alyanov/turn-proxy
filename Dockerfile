FROM rust:1.94-slim AS builder
LABEL authors="artemos"

WORKDIR /usr/src/turn-proxy
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/turn-proxy/target/release/turn-proxy-server /usr/local/bin/turn-proxy-server
RUN mkdir -p /etc/turn-proxy/server/

EXPOSE 56000/udp

ENTRYPOINT ["turn-proxy-server"]
CMD ["--config", "/etc/turn-proxy/server/config.toml"]