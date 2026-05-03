.PHONY: assets assetsdir package wasm publish

wasm:
	RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --target web --release --no-default-features -- --features console_error_panic_hook
	cd pkg && bun link

publish:
	wasm-pack publish
