default:
	cargo build --release

test:
	RUST_BACKTRACE=1 cargo test
