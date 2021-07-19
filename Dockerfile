FROM paritytech/ci-linux:1a002583-20210719 AS BASE
WORKDIR /build

ENV CARGO_HOME=/build/node-template/.cargo

COPY . .

RUN cargo build --release

FROM paritytech/ci-linux:1a002583-20210719
WORKDIR /node

COPY --from=BASE /build/target/release/node-template .

CMD [ "/node/node-template", "--dev", "--ws-external" ]
