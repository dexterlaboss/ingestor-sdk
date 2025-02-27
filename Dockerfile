FROM rust:1.84-slim-bullseye AS build

RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y --no-install-recommends \
    apt-utils \
    software-properties-common \
    cmake \
    build-essential \
    libclang-dev \
    libudev-dev \
    libssl-dev \
    libsasl2-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN USER=root cargo new --bin solana
WORKDIR /solana

COPY . /solana

RUN cargo build --release



FROM debian:bullseye-slim

RUN mkdir -p /solana

WORKDIR /solana

COPY --from=build /solana/target/release/ingestor-kafka-service .

EXPOSE 8899

ENV RUST_LOG=info
CMD ["./ingestor-kafka-service"]