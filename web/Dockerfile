FROM rust:1.75
RUN apt-get update
RUN apt-get install -y libssl-dev ca-certificates
WORKDIR /usr/src/blog
COPY .. /usr/src/blog
RUN cargo build --release
COPY ./analytics ./analytics
COPY ./articles ./articles
ENTRYPOINT ["/usr/src/blog/target/release/blog"]