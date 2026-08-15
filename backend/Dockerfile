FROM rust:1-bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
COPY migrations ./migrations
RUN cargo build --locked --release -p knitprint-api --bin knitprint-api

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home knitprint
COPY --from=build /build/target/release/knitprint-api /usr/local/bin/knitprint-api
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/knitprint-api"]
