FROM paritytech/ci-linux:d12bfad6-20210720 AS BASE
WORKDIR /build

ENV CARGO_HOME=/build/node-template/.cargo

COPY . .

RUN cargo build --release

FROM paritytech/ci-linux:d12bfad6-20210720
WORKDIR /node

COPY --from=BASE /build/target/release/node-template .

CMD [ "/node/node-template", "--dev", "--ws-external" ]
