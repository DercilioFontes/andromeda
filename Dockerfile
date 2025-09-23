FROM rust:trixie

WORKDIR /usr/src/myapp

RUN rustup install nightly && rustup default nightly
RUN cargo install --git https://github.com/tryandromeda/andromeda andromeda

CMD ["andromeda"]
