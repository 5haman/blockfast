NAME=blockfast

.PHONY: test

default: debug

release:
	cargo build --release
	strip ./target/release/$(NAME)

debug:
	cargo build

build:
	cargo build --release

test:
	RUST_BACKTRACE=full time -l target/debug/$(NAME) -b test/blocks

install:
	cp ./target/release/$(NAME) /usr/local/bin/$(NAME)

init:
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile default --default-toolchain default
	source ~/.cargo/env
	rustup target add x86_64-unknown-linux-musl
	ln -s /usr/bin/gcc /usr/bin/musl-gcc
