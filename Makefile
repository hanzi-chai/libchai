.PHONY: assets assetsdir package

wasm:
	wasm-pack build --target web

publish:
	wasm-pack publish
