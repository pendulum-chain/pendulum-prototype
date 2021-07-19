FROM paritytech/ci-linux:1a002583-20210719 AS BASE
WORKDIR /build

ENV CARGO_HOME=/build/node-template/.cargo

COPY . .

RUN rustup toolchain uninstall nightly-x86_64-apple-darwin
RUN rustup toolchain install nightly
RUN rustup target add wasm32-unknown-unknown
RUN cargo build --release

FROM paritytech/ci-linux:1a002583-20210719
WORKDIR /node

COPY --from=BASE /build/target/release/node-template .

CMD [ "/node/node-template", "--dev", "--ws-external" ]
