FROM rust:1.67

WORKDIR /usr/src/blog
COPY . .

RUN cargo install --path .

CMD ["blog"]
