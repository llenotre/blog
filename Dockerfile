FROM rust:1.69

WORKDIR /usr/src/blog
COPY . .
RUN cargo build --release

CMD ["target/release/blog"]
