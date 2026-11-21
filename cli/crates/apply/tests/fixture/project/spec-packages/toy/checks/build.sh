# Every note starts with "note ".
for f in notes/*.txt; do
  [ -e "$f" ] || continue
  head -c 5 "$f" | grep -q '^note ' || { echo "$f does not start with 'note '"; exit 1; }
done
exit 0
