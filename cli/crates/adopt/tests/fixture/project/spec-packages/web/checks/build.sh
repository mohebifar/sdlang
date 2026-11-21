# Every page has a default export.
for f in $(find src/app -name page.tsx); do
  grep -q "export default" "$f" || { echo "$f has no default export"; exit 1; }
done
exit 0
