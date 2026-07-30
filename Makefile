.PHONY: check install-local

check:
	cargo fmt --check
	cargo clippy --locked --all-targets -- -D warnings
	cargo test --locked

install-local:
	cargo build --release --locked
	mkdir -p "$(HOME)/.local/bin"
	cp target/release/kj "$(HOME)/.local/bin/kj"
