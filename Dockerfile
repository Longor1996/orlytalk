FROM node as ts-builder
WORKDIR /usr/src/orlytalk
RUN npm install -g typescript
COPY . .
RUN tsc -p orly-server/src/www/ts/tsconfig.production.json

FROM rust as rust-builder
WORKDIR /usr/src/orlytalk
COPY --from=ts-builder /usr/src/orlytalk .
RUN cargo install --path orly-server

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=rust-builder /usr/local/cargo/bin/orlytalk/orly-server /usr/local/bin/orlytalk/orly-server
CMD ["orly-server"]
