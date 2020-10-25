FROM rust:1.47 AS cacher
LABEL Name=udp2amqp Version=0.1.0

WORKDIR /app
RUN cargo install cargo-chef
COPY ./recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.47 AS builder
WORKDIR /app
COPY --from=cacher /app/target ./target
COPY ./Cargo.lock ./Cargo.toml ./
COPY ./src ./src
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/udp2amqp .
USER 1000

ENTRYPOINT [ "./udp2amqp" ]
