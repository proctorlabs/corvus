# Build container
FROM rust:1.47-alpine3.12 as builder
RUN mkdir -p /work/src
WORKDIR /work
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN apk add musl-dev musl-utils
ADD ["Cargo.toml", "Cargo.lock", "./"]
RUN cargo build --release && rm ./target/release/corvus ./target/release/deps/corvus-*
ADD ./src/ ./src/
RUN cargo build --release

# Target container
FROM alpine:3.12
COPY --from=builder /work/target/release/corvus /bin/corvus

# Admin capacity required for bluetooth
# e.g. docker run -it --rm -v $PWD/corvus.toml:/corvus.toml --cap-add=NET_ADMIN --net=host <container> corvus -v
