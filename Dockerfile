# Based on <https://www.lpalmieri.com/posts/fast-rust-docker-builds/>

FROM paritytech/ci-linux:974ba3ac-20201006 AS planner
WORKDIR /build

ENV CARGO_HOME=/build/node-template/.cargo

RUN cargo install cargo-chef

COPY . .

RUN cargo chef prepare --recipe-path recipe.json


FROM paritytech/ci-linux:974ba3ac-20201006 AS cacher
WORKDIR /build

ENV CARGO_HOME=/build/node-template/.cargo

RUN cargo install cargo-chef

COPY --from=planner /build/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json


FROM paritytech/ci-linux:974ba3ac-20201006 AS builder
WORKDIR /build

COPY . .
# Copy over the cached dependencies
COPY --from=cacher /build/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

RUN cargo build --release


FROM paritytech/ci-linux:974ba3ac-20201006
WORKDIR /node

COPY --from=builder /build/target/release/node-template .

CMD [ "/node/node-template", "--dev", "--ws-external" ]

