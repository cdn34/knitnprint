FROM rust:1-bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS build
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

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home knitnprint
COPY --from=build \
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
USER 10001:10001
EXPOSE 8080
CMD ["/usr/local/bin/knitnprint-api"]
