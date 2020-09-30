FROM node as www-builder
WORKDIR /usr/src/orlytalk
RUN npm install -g parcel-bundler
COPY ./orly-client-web/ ./orly-client-web/
COPY ./package-lock.json ./package-lock.json
COPY ./package.json ./package.json
RUN npm update
RUN npm run build

FROM rust as rust-builder
WORKDIR /usr/src/orlytalk
COPY ./orly-server/ ./orly-server/
COPY ./Cargo.toml ./Cargo.toml
RUN cargo install --path orly-server

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=rust-builder /usr/local/cargo/bin/orly-server /usr/local/bin/orly-server
COPY --from=www-builder /usr/src/orlytalk/orly-client-web/out/ /usr/local/bin/orly-server-www/

HEALTHCHECK --interval=5m --timeout=1s CMD curl -f http://localhost:6991/ || exit 1
EXPOSE 6991/tcp
CMD ["orly-server"]
