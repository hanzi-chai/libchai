.PHONY: assets assetsdir package

all: build

assets: assetsdir character_frequency.txt word_frequency.txt equivalence.txt

assetsdir:
	mkdir -p assets

%.txt:
	curl "https://assets.chaifen.app/$@" -o assets/$@

package: build build-windows
	mkdir -p package; \
	cp target/release/libchai package/; \
	cp target/x86_64-pc-windows-gnu/release/libchai.exe package/; \
	cp -r README.md LICENSE config.yaml elements.txt assets package/; \
	cd package; \
	rm libchai.zip; \
	zip -r libchai.zip *

build:
	cargo build

build-windows:
	cargo build --release --target x86_64-pc-windows-gnu

build-linux:
	cargo build --release --target x86_64-unknown-linux-gnu
