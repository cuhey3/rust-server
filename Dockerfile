# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM rust:1.78

# Copy local code to the container image.
WORKDIR /usr/src/app
COPY . .

# Install production dependencies and build a release artifact.
RUN cd ./my-wasm && cargo install wasm-pack && wasm-pack build --target web && cd ../ && cargo install --path .

# Run the web service on container startup.
CMD ["rust-server"]