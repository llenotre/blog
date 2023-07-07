FROM rust:1.70 AS base
RUN cargo install cargo-chef --locked
WORKDIR /usr/src/blog

FROM base AS prepare
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base as build
COPY --from=prepare /usr/src/blog/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:buster-slim AS runtime
WORKDIR /usr/src/blog
COPY --from=build /usr/src/blog/target/release/blog /usr/local/bin
ENTRYPOINT ["/usr/local/bin/blog"]
