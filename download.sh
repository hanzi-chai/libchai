mkdir -p assets

for asset in "character_elements" "character_frequency" "word_elements" "word_frequency" "equivalence" "map" "fixed_map"; do
  curl "https://assets.chaifen.app/${asset}.txt" -o assets/${asset}.txt
done
