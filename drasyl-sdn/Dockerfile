FROM rust:1.86-bullseye AS build
COPY . /build
WORKDIR /build/
RUN cargo install --path drasyl-sdn

FROM debian:bullseye-slim
COPY --from=build /usr/local/cargo/bin/drasyl /usr/local/bin/drasyl
ENV DRASYL_UDP_PORT="22527"
EXPOSE 22527/udp 22527/tcp
WORKDIR /drasyl/
ENTRYPOINT ["/usr/local/bin/drasyl"]