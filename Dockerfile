FROM rust
WORKDIR /tmp/app
ADD Cargo.toml .
ADD Cargo.lock .
ADD dummy.rs .
RUN cargo build --lib
ADD src /tmp/app/src
ADD sample_meeting .
RUN cargo build
CMD cargo test
