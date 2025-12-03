FROM rust:1.88 AS libcrypto-builder
WORKDIR /src

RUN mkdir src && touch src/lib.rs
COPY ./libcrypto/Cargo.toml ./libcrypto/Cargo.lock .
RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  cargo fetch

COPY ./libcrypto/src/ ./src/
RUN \
  --mount=type=cache,target=/usr/local/cargo/registry \
  cargo build --release --frozen --target-dir /artifact

FROM golang:1.25.2-bookworm AS broker-builder
WORKDIR /src

COPY ./go.mod ./go.sum .
RUN \
    --mount=type=cache,target=/go/pkg/mod \
    go mod download

COPY --from=libcrypto-builder /artifact/release/libcrypto.a ./internal/common/crypto/libcrypto.a
COPY . .
RUN \
    --mount=type=cache,target=/go/pkg/mod \
    go build -ldflags "-s -w" -mod=readonly -o=/artifact/program ./cmd/rest-api/

FROM cgr.dev/chainguard/wolfi-base:latest
WORKDIR /bin

COPY --from=broker-builder --chown=nonroot:nonroot /artifact/program /bin
USER nonroot

# see https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry#labelling-container-images
LABEL "org.opencontainers.image.source"="https://github.com/mirzahilmi/secure_ldr_pir"
LABEL "org.opencontainers.image.description"="🔒 Secured LDR & PIR in transit sensor readings"
LABEL "org.opencontainers.image.licenses"="AGPL-3.0"

ENTRYPOINT ["/bin/program"]
