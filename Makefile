.PHONY: check install-local package-release test-installer

check:
	cargo fmt --check
	cargo clippy --locked --all-targets -- -D warnings
	cargo test --locked

install-local:
	cargo build --release --locked
	mkdir -p "$(HOME)/.local/bin"
	cp target/release/kj "$(HOME)/.local/bin/kj"

package-release:
	./scripts/package-release.sh "$(TARGET)" "$(DIST_DIR)"

test-installer:
	./scripts/test-installer.sh
