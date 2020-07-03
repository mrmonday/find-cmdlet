FROM rust:1.44.1-buster

RUN \
    apt-get -y update && \
    apt-get -y install nodejs npm && \
    npm install -g terser@4.6.13 && \
    npm install -g typescript@3.8.3 && \
    npm install -g sass@1.26.5 && \
    cargo install cargo-deb && \
    chmod -R a+rw $CARGO_HOME
