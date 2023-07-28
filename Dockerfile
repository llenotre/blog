FROM rust:1.70 AS base
RUN cargo install cargo-chef --locked
WORKDIR /usr/src/blog

# Prepare
FROM base AS prepare
COPY src/ src/
COPY pages/ pages/
COPY analytics/ analytics/
ADD Cargo.toml .
ADD Cargo.lock .
RUN cargo chef prepare --recipe-path recipe.json

# Build
FROM base as build
COPY --from=prepare /usr/src/blog/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY src/ src/
COPY pages/ pages/
ADD Cargo.toml .
ADD Cargo.lock .
ADD config.toml .
RUN cargo build --release

# Prepare runtime
FROM debian:bullseye-slim AS runtime
RUN apt-get update
RUN apt-get install -y libssl-dev ca-certificates
WORKDIR /usr/src/blog
COPY --from=build /usr/src/blog/target/release/blog /usr/local/bin
COPY assets/ assets/
COPY analytics/ analytics/
ADD config.toml .
ENTRYPOINT ["/usr/local/bin/blog"]
