FROM paritytech/ci-linux:d12bfad6-20210720 AS BASE
WORKDIR /build

ENV CARGO_HOME=/build/pendulum-node/.cargo

COPY . .

RUN cargo build --release

FROM paritytech/ci-linux:d12bfad6-20210720
WORKDIR /node

COPY --from=BASE /build/target/release/pendulum-node .

CMD [ "/node/pendulum-node", "--dev", "--ws-external" ]
