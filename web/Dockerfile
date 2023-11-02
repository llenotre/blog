FROM rust:1.73 AS base
RUN cargo install cargo-chef --locked

# Prepare
FROM base AS prepare
WORKDIR /usr/src/blog
COPY .. .
RUN cargo chef prepare --recipe-path recipe.json

# Build
FROM base as build
WORKDIR /usr/src/blog
COPY .. .
COPY --from=prepare /usr/src/blog/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo build --release

# Prepare runtime
FROM debian:bullseye-slim AS runtime
RUN apt-get update
RUN apt-get install -y libssl-dev ca-certificates
WORKDIR /usr/src/blog
COPY --from=build /usr/src/blog/target/release/blog /usr/local/bin/
COPY assets assets/
COPY analytics analytics/
COPY config.toml .
ENTRYPOINT ["/usr/local/bin/blog"]