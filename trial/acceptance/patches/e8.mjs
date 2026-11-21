// The E8 hand patch (TRIAL.md, E8), applied by the operator to every arm before E8:
// the About heading becomes "About this app" and a paragraph "Built for the int lang
// trial." follows it. Usage: node patches/e8.mjs <arm dir> <where to save the patched copy>
import fs from "node:fs";
import path from "node:path";

const [arm, save] = process.argv.slice(2);
if (!arm || !save) {
  console.error("usage: node patches/e8.mjs <arm dir> <save path>");
  process.exit(2);
}
const file = path.join(arm, "src/app/about/page.tsx");
const text = fs.readFileSync(file, "utf8");
const heading = /<h1>\s*About\s*<\/h1>/g;
const found = text.match(heading) ?? [];
if (found.length !== 1) {
  console.error(`${file}: expected exactly one <h1>About</h1>, found ${found.length}; patch by hand`);
  process.exit(1);
}
const indent = (text.split("\n").find((l) => heading.test(l)) ?? "").match(/^\s*/)[0];
heading.lastIndex = 0;
const patched = text.replace(
  heading,
  `<h1>About this app</h1>\n${indent}<p>Built for the int lang trial.</p>`,
);
fs.writeFileSync(file, patched);
fs.mkdirSync(path.dirname(save), { recursive: true });
fs.writeFileSync(save, patched);
console.log(`patched ${file}; saved a copy to ${save}`);
