.PHONY: test
test:
	RUSTFLAGS="-D warnings" cargo build
	RUSTFLAGS="-D warnings" cargo test
	RUSTFLAGS="-D warnings" cargo clippy
	cargo fmt --check
