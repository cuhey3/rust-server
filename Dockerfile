# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM rust:1.78

WORKDIR /usr/src/app/my-wasm
COPY . .

RUN cargo install wasm-pack
RUN wasm-pack build --target web

# Copy local code to the container image.
WORKDIR /usr/src/app
COPY . .

# Install production dependencies and build a release artifact.
RUN cargo install --path .

# Run the web service on container startup.
CMD ["rust-server"]