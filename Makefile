.PHONY: assets assetsdir package

wasm:
	RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --target web

publish:
	wasm-pack publish
