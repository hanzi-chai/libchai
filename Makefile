.PHONY: assets assetsdir package

all: build

assets: assetsdir character_frequency.txt word_frequency.txt key_equivalence.txt pair_equivalence.txt

assetsdir:
	mkdir -p assets

%.txt:
	curl "https://assets.chaifen.app/$@" -o assets/$@

package: build build-windows
	mkdir -p package; \
	cp target/release/chai package/; \
	cp target/x86_64-pc-windows-gnu/release/chai.exe package/; \
	cp -r README.md config.md LICENSE config.yaml elements.txt assets package/; \
	cd package; \
	rm chai.zip; \
	zip -r chai.zip *

build:
	cargo build --release

build-windows:
	cargo build --release --target x86_64-pc-windows-gnu

wasm:
	wasm-pack build --target web

publish:
	wasm-pack publish
