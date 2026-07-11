FROM rust:1.92-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p f1-api

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/f1-api /usr/local/bin/f1-api
ENV API_HOST=0.0.0.0
ENV API_PORT=8080
EXPOSE 8080
CMD ["f1-api"]
