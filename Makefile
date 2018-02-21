all: bin
	docker build --no-cache -t  lcharignon/optirust:latest .

bin:
	docker run --rm -it -v "$$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release

push:
	docker push lcharignon/optirust:latest


