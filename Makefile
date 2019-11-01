TARGET=musl

.PHONY: test

default: build

release:
	cargo build --release --target=x86_64-unknown-linux-$(TARGET)
	strip ./target/x86_64-unknown-linux-$(TARGET)/release/parser

debug:
	cargo build --debug --target=x86_64-unknown-linux-$(TARGET)

build:
	cargo build --release

test:
	time -l target/release/blockfast -b test/blocks

install:
	cp ./target/x86_64-unknown-linux-$(TARGET)/release/parser /usr/bin/parser

init:
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile default --default-toolchain default
	source ~/.cargo/env
	rustup target add x86_64-unknown-linux-musl
	ln -s /usr/bin/gcc /usr/bin/musl-gcc
