FROM rust:1.73
RUN apt-get update
RUN apt-get install -y libssl-dev ca-certificates
WORKDIR /usr/src/blog
COPY .. .
RUN cargo build --release
ENTRYPOINT ["/usr/src/blog/target/release/blog"]