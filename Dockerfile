FROM rust:1.45 as builder
WORKDIR /usr/src/toolit-api
COPY . .
RUN cargo install --path . --locked

FROM debian:buster-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates openssl
COPY --from=builder /usr/local/cargo/bin/toolit-api /usr/local/bin/toolit-api
#Copy the relevant swish files
COPY --from=builder /usr/src/toolit-api/toolit_swish.p12 ./toolit_swish.p12
#Copy the relevant bankid files
COPY --from=builder /usr/src/toolit-api/trust_server_certificate.txt ./trust_server_certificate.txt
COPY --from=builder /usr/src/toolit-api/Keystore_Toolit_20210420.p12 ./Keystore_Toolit_20210420.p12
EXPOSE 3030
CMD ["toolit-api"]
