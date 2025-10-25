FROM docker.io/rust:1-trixie AS builder
RUN apt -y update
RUN apt -y install clang
RUN mkdir -p /module/src
COPY /Cargo.* /module/
ADD /src/ /module/src
WORKDIR /module
RUN cargo build

FROM docker.io/nginx:1.28
RUN apt -y update
RUN apt -y install python3
COPY --from=builder /module/target/debug/libpayload_filter.so /etc/nginx/libpayload_filter.so
COPY /build/server.py /server.py
COPY /build/nginx.conf /etc/nginx/nginx.conf
COPY /build/start.sh /start.sh
RUN chmod 755 /start.sh
EXPOSE 8080

ENTRYPOINT /start.sh

