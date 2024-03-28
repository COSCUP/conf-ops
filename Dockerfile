# build frontend
FROM node:lts as frontend-builder

WORKDIR /usr/src/app

RUN corepack enable pnpm
RUN corepack use pnpm@8

RUN mkdir -p client
COPY client/package.json client/pnpm-lock.yaml  ./client/
RUN cd client && pnpm install --frozen-lockfile

COPY client ./client
RUN cd client && pnpm build

# build backend
FROM rust:1.76 as backend-builder

WORKDIR /usr/src/app

COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
RUN mkdir src && touch src/lib.rs
RUN cargo build --release

COPY ./src ./src
RUN cargo build --release

# application
FROM debian:bookworm-slim

WORKDIR /usr/src/app
RUN apt-get update && apt-get install -y libssl-dev libmariadb-dev && rm -rf /var/lib/apt/lists/*

COPY ./migrations ./migrations
COPY README.md README.md
COPY LICENSE LICENSE

RUN mkdir -p public
COPY --from=frontend-builder /usr/src/app/public ./public

COPY --from=backend-builder /usr/src/app/target/release/conf-ops ./

CMD ["./conf-ops"]
