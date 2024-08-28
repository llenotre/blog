FROM rust:1.75

RUN apt-get update
RUN apt-get install -y libssl-dev ca-certificates

WORKDIR /usr/src/blog

COPY ./src ./src
COPY ./macros ./macros
COPY ./pages ./pages
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release

COPY ./analytics ./analytics
COPY ./articles ./articles
COPY ./assets ./assets
COPY ./update.sh ./update.sh

ENTRYPOINT ["/usr/src/blog/target/release/blog"]