FROM buildpack-deps:trusty
ADD ./target/x86_64-unknown-linux-musl/release/optirust /
ADD cbc /
ENV PATH="/:${PATH}"
ENTRYPOINT ["/optirust"]
