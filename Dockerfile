FROM rust:1.75 as builder

COPY ./Cargo.toml /app/Cargo.toml
COPY ./Cargo.lock /app/Cargo.lock
COPY ./src /app/src

WORKDIR /app

RUN cargo build --release

FROM ubuntu:23.10 as final

COPY --from=builder /app/target/release/rust-async-disk-api /app/rust-async-disk-api

WORKDIR /app

CMD ["./rust-async-disk-api"]
