FROM rust:1.68.2 as builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:buster-slim

WORKDIR /app

COPY --from=builder /app/target/release/rustdress /app

CMD ["./rustdress"]