# Use the official Rust image.
# https://hub.docker.com/_/rust
FROM rust:1.78

# Copy local code to the container image.
WORKDIR /usr
COPY . ./src/app

# Install production dependencies and build a release artifact.
RUN cargo install --path ./src/app

# Run the web service on container startup.
CMD ["rust-server"]