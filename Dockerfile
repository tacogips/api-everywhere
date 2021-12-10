ARG RUST_VER=1.57

# --- prepare ------------------------------------------------
From rust:${RUST_VER} as prepare
RUN cargo install cargo-chef && rm -rf $CARGO_HOME/registry/

WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- cacher -----------------------------------------------
From rust:${RUST_VER} as cacher
RUN cargo install cargo-chef && rm -rf $CARGO_HOME/registry/

WORKDIR /app
COPY --from=prepare /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

# --- builder -----------------------------------------------
From rust:${RUST_VER} as builder
ARG BUILD_FLAG=--release

RUN rustup component add rustfmt
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME

WORKDIR /app/api-everywhere
RUN cargo build ${BUILD_FLAG}

# --- bin -----------------------------------------------
From debian:buster-slim as runtime
ARG BUILD_TARGET=release
ARG SERVICE_ACCOUNT_FILE=./dev-secret/test-sa-key.json
ARG PLAYGROUND_DIR=./src/playground_html

# to fix:
# error while loading shared libraries: libssl.so.1.1: cannot open shared object file: No such file or directory
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates

#RUN apt-get -y update && apt-get install -y libssl-dev
WORKDIR /app

COPY --from=builder /app/target/${BUILD_TARGET}/api-everywhere api-everywhere

COPY ${SERVICE_ACCOUNT_FILE} app_service_account.json
COPY ${PLAYGROUND_DIR} playground_html

RUN ["chmod","+x","./api-everywhere"]
RUN ["ls"]

ENV SERVICE_ACCONT_FILE=/app/app_service_account.json
ENV PLAYGROUND_DIR=/app/playground_html

CMD ["./api-everywhere","--host","0.0.0.0"]
