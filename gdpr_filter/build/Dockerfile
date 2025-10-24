FROM docker.io/rust:1-trixie AS builder
RUN apt -y update
RUN apt -y install clang
RUN mkdir -p /module/src
COPY /Cargo.* /module/
ADD /src/ /module/src
WORKDIR /module
RUN cargo build

FROM docker.io/nginx:1.28
COPY --from=builder /module/target/debug/libgdpr_filter.so /etc/nginx/libgdpr_filter.so
COPY /build/nginx_before.conf /etc/nginx/nginx1.conf
COPY /build/nginx_after.conf /etc/nginx/nginx2.conf
RUN mkdir -p /var/www/website
ADD /build/website /var/www/website
COPY /build/start.sh /start.sh
RUN chmod 755 /start.sh
EXPOSE 8080
EXPOSE 8081

ENTRYPOINT /start.sh

