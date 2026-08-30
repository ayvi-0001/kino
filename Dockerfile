FROM rust:1.94-slim AS builder

ENV SHELL=/usr/bin/bash

RUN apt-get update && \
  apt-get install -y --no-install-recommends git && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

ENV SQLX_OFFLINE=true

RUN cargo build --features postgres --release --locked

FROM debian:trixie-slim AS runtime

USER root

ENV SHELL=/usr/bin/bash

# Ensure apt-get doesn't open a menu
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update -y \
  && apt-get install -y --no-install-recommends ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kino /usr/local/bin/kino

VOLUME /data

CMD ["kino"]
