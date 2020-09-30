FROM node as www-builder
WORKDIR /usr/src/orlytalk
RUN npm install -g parcel-bundler
COPY . .
RUN npm update
RUN npm run build

FROM rust as rust-builder
WORKDIR /usr/src/orlytalk
COPY --from=www-builder /usr/src/orlytalk .
RUN cargo install --path orly-server

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=rust-builder /usr/local/cargo/bin/orly-server /usr/local/bin/orly-server
CMD ["orly-server"]
