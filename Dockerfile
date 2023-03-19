FROM rust:slim AS builder
WORKDIR /app
RUN apt update && apt install lld clang -y
ADD ZscalerRootCertificate-2048-SHA256.crt /usr/local/share/ca-certificates/ca-certificate.crt
RUN apt-get -y install ca-certificates libssl-dev
RUN chmod 644 /usr/local/share/ca-certificates/ca-certificate.crt && update-ca-certificates
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release

FROM debian:bullseye-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*
# Copy compile binary from the builder step to runtime step
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENV production
ENTRYPOINT ["./zero2prod"]
