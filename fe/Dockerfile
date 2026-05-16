FROM rust:1.85-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk --version 0.21.5

COPY . .

# API_URL is baked into the WASM binary at compile time via build.rs reading .env
ARG API_URL=http://localhost:3000
RUN printf "API_URL=%s\n" "$API_URL" > .env

RUN trunk build --release

FROM nginx:1.27-alpine AS runtime
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
