FROM rust:1.88 as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release --bin kora

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/kora /usr/local/bin/

EXPOSE 8080
CMD ["kora"]