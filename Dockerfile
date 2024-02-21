FROM rust:1.75 as builder

COPY ./Cargo.toml /app/Cargo.toml
COPY ./Cargo.lock /app/Cargo.lock
COPY ./.sqlx /app/.sqlx

COPY ./src /app/src
COPY ./.env /app/.env

WORKDIR /app

ARG SQLX_OFFLINE=true

RUN cargo build --release

FROM ubuntu:23.10 as final

COPY --from=builder /app/target/release/rust-async-disk-api /app/rust-async-disk-api
COPY --from=builder /app/.env /app/.env

WORKDIR /app

CMD ["./rust-async-disk-api"]
