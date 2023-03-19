FROM rust:latest

WORKDIR /app
RUN apt update && apt install lld clang -y
ADD ZscalerRootCertificate-2048-SHA256.crt /usr/local/share/ca-certificates/ca-certificate.crt
RUN apt-get -y install ca-certificates libssl-dev
RUN chmod 644 /usr/local/share/ca-certificates/ca-certificate.crt && update-ca-certificates
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release
ENTRYPOINT ["./target/release/zero2prod"]
