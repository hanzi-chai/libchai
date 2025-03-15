.PHONY: assets assetsdir package

fetch: # 冰雪四拼.yaml 冰雪四拼.txt 米十五笔.yaml 米十五笔.txt
	mkdir -p examples; \
	mkdir -p assets; \
	for file in key_distribution.txt pair_equivalence.txt; do \
		curl "https://assets.chaifen.app/$$file" -o assets/$$file; \
	done; \
	for file in 冰雪四拼.yaml 冰雪四拼.txt 米十五笔.yaml 米十五笔.txt; do \
		curl "https://assets.chaifen.app/$$file" -o examples/$$file; \
	done

wasm:
	wasm-pack build --target web

publish:
	wasm-pack publish
