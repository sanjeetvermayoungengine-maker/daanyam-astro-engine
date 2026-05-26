FROM rust:1.86-bookworm AS builder
WORKDIR /app

ARG GIT_COMMIT=unknown
ARG BUILD_DATE
ENV GIT_COMMIT=${GIT_COMMIT}
ENV BUILD_DATE=${BUILD_DATE}

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tests ./tests
COPY docs ./docs
COPY benches ./benches
COPY README.md RELEASE.md rustfmt.toml ./

RUN BUILD_DATE="${BUILD_DATE:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}" \
    cargo build -p astro-api --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

ENV HOST=0.0.0.0
ENV PORT=3000
ENV ASTRO_BACKEND=de440
ENV ASTRO_EPHE_PATH=/tmp/ephe/de440.bsp
ENV ASTRO_EPHE_CACHE_DIR=/tmp/ephe

WORKDIR /app
COPY --from=builder /app/target/release/astro-api /usr/local/bin/astro-api
RUN mkdir -p /tmp/ephe

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail http://127.0.0.1:${PORT}/health || exit 1

CMD ["/usr/local/bin/astro-api"]
