# syntax=docker/dockerfile:1
# cna-server and the browser client in one image (Mission 4 spec, *Image*).
#
# The rules data comes from basmith7/cna at CNA_RULES_COMMIT, fetched here
# rather than taken from the build context: the shared tag build
# (basmith7/workflows build-image.yml) checks out without submodules.
#   docker build --build-arg CNA_RULES_COMMIT=$(git -C vendor/cna rev-parse HEAD) -t cna:dev .

FROM rust:1-trixie AS build
ARG CNA_RULES_COMMIT
ARG WASM_PACK_VERSION=0.15.0
ARG WASM_PACK_SHA256=c09f971ecaed9a2efc80fdcea7a00ef6b53c7fadc8c57d1f61b53a6aa66b668a
RUN test -n "$CNA_RULES_COMMIT" || { echo "build arg CNA_RULES_COMMIT is required" >&2; exit 1; }
COPY --from=node:24-trixie-slim /usr/local/bin/node /usr/local/bin/node
COPY --from=node:24-trixie-slim /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
 && rustup target add wasm32-unknown-unknown \
 && curl -fsSL -o /tmp/wasm-pack.tgz "https://github.com/wasm-bindgen/wasm-pack/releases/download/v${WASM_PACK_VERSION}/wasm-pack-v${WASM_PACK_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
 && echo "${WASM_PACK_SHA256}  /tmp/wasm-pack.tgz" | sha256sum -c - \
 && tar -xzf /tmp/wasm-pack.tgz --strip-components=1 -C /usr/local/bin "wasm-pack-v${WASM_PACK_VERSION}-x86_64-unknown-linux-musl/wasm-pack"
WORKDIR /src
COPY . .
RUN git init -q vendor/cna \
 && git -C vendor/cna fetch -q --depth 1 https://github.com/basmith7/cna.git "$CNA_RULES_COMMIT" \
 && git -C vendor/cna checkout -q FETCH_HEAD
ENV CNA_RULES_COMMIT=$CNA_RULES_COMMIT
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p cna-server --bin cna-server \
 && cp target/release/cna-server /usr/local/bin/cna-server
WORKDIR /src/web
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    --mount=type=cache,target=/root/.npm \
    npm ci && npm run wasm && npm run build

FROM debian:trixie-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home cna
COPY --from=build /usr/local/bin/cna-server /usr/local/bin/cna-server
COPY --from=build /src/web/dist /app/web
ENV CNA_WEB_DIST=/app/web CNA_BIND=0.0.0.0:3240
USER cna
EXPOSE 3240
# /healthz checks the database too.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD curl -fsS http://127.0.0.1:3240/healthz || exit 1
CMD ["cna-server"]
