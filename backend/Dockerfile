FROM debian:bookworm-slim@sha256:99c0b25c46d0214d5a2091092e7908907dcaecd6a962cde0ac700acee222b6f3 AS runtime-security
WORKDIR /packages
# The pinned distroless image predates the fix for CVE-2026-84782. Extract the
# signed Debian security package, including its package metadata, into an overlay.
RUN apt-get update \
    && apt-get download libssl3=3.0.22-1~deb12u2 \
    && mkdir -p /runtime/var/lib/dpkg/status.d \
    && dpkg-deb --extract libssl3_*.deb /runtime \
    && dpkg-deb --control libssl3_*.deb /control \
    && cp /control/control /runtime/var/lib/dpkg/status.d/libssl3 \
    && cp /control/md5sums /runtime/var/lib/dpkg/status.d/libssl3.md5sums

FROM rust:1-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
COPY migrations ./migrations
RUN cargo build --locked --release -p knitnprint-api \
    --bin knitnprint-api \
    --bin migrate \
    --bin create_owner \
    --bin deliver_notifications \
    --bin cleanup_sessions \
    --bin cleanup_customers \
    --bin cleanup_carts \
    --bin cleanup_media \
    --bin cleanup_payments \
    --bin check_operations

FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f AS runtime-base
COPY --from=runtime-security /runtime/ /

FROM runtime-base
COPY --from=build --chown=65532:65532 \
    /build/target/release/knitnprint-api \
    /build/target/release/migrate \
    /build/target/release/create_owner \
    /build/target/release/deliver_notifications \
    /build/target/release/cleanup_sessions \
    /build/target/release/cleanup_customers \
    /build/target/release/cleanup_carts \
    /build/target/release/cleanup_media \
    /build/target/release/cleanup_payments \
    /build/target/release/check_operations \
    /usr/local/bin/
USER 65532:65532
EXPOSE 8080
CMD ["/usr/local/bin/knitnprint-api"]
