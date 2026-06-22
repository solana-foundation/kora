# cargo-chef splits dependency compilation into a layer keyed only on the dependency manifest
# (recipe.json), so app-source changes reuse the cached dep build instead of recompiling everything.
FROM rust:1.89 AS chef
RUN cargo install cargo-chef --locked
WORKDIR /usr/src/app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /usr/src/app/recipe.json recipe.json
# Cache-stable: only rebuilds when dependencies change, not on app-source edits.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin kora

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/kora /usr/local/bin/

EXPOSE 8080
CMD ["kora"]
