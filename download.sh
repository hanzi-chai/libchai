mkdir -p assets

for asset in "character_frequency" "word_frequency" "equivalence"; do
  curl "https://assets.chaifen.app/${asset}.txt" -o assets/${asset}.txt
done
