.PHONY: assets assetsdir package wasm publish

wasm:
	wasm-pack build --target web --release --no-default-features -- --features console_error_panic_hook

publish:
	wasm-pack publish
