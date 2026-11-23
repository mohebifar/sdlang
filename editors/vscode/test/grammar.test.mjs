// Tokenizes sdlang samples with the TextMate grammar and checks the scopes of chosen
// tokens: `npm test`.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import vsctm from "vscode-textmate";
import oniguruma from "vscode-oniguruma";

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const wasm = fs.readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm")).buffer;
await oniguruma.loadWASM(wasm);

const grammars = {
  "source.sdlang": "../syntaxes/sdlang.tmLanguage.json",
  "markdown.sdlang.codeblock": "../syntaxes/sdlang.markdown.tmLanguage.json",
};
const registry = new vsctm.Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (s) => new oniguruma.OnigScanner(s),
    createOnigString: (s) => new oniguruma.OnigString(s),
  }),
  loadGrammar: async (scope) => {
    const file = grammars[scope];
    if (!file) return null;
    const p = path.join(here, file);
    return vsctm.parseRawGrammar(fs.readFileSync(p, "utf8"), p);
  },
});
const grammar = await registry.loadGrammar("source.sdlang");

/** Each line's tokens as [text, scopes]. */
function tokenize(source) {
  let stack = vsctm.INITIAL;
  return source.split("\n").map((line) => {
    const r = grammar.tokenizeLine(line, stack);
    stack = r.ruleStack;
    return r.tokens.map((t) => [line.slice(t.startIndex, t.endIndex), t.scopes]);
  });
}

const sample = `import nextjs
import package "./spec-packages/rust" as rust
use "drizzle" for database { dialect: "postgres" }
moved shop.add -> shop.add_to_cart

// A page and its action.
export nextjs.page product_page {
  route: "/products/[id]"
  calls: [add_to_cart]
  prompt: """
    Show the price. See @{shop.cart} and \${route}.
  """
}

action add_to_cart {
  input: { id: string, quantity: number }
  output: { items: integer }
  uses: [cart]
  expect {
    adds_item: "cart has the product"
  }
}

export type Plan = "free" | "team"
let limit = 3
extern component Form {
  source: "ui-sdk#Form"
}
export kind page {
  fields {
    calls: [ref action] @relation("calls")
  }
}
rule pages {
  applies_to: select nextjs.page where route matches "/app/**"
}`;

const lines = tokenize(sample);
function scopes(lineNo, text, nth = 0) {
  const hits = lines[lineNo].filter(([t]) => t.trim() === text);
  assert.ok(hits.length > nth, `line ${lineNo + 1}: no token '${text}' in ${JSON.stringify(lines[lineNo].map((t) => t[0]))}`);
  return hits[nth][1].join(" ");
}
function expect(lineNo, text, scope, nth = 0) {
  const s = scopes(lineNo, text, nth);
  assert.ok(s.includes(scope), `line ${lineNo + 1}, '${text}': expected ${scope}, got ${s}`);
}

expect(0, "import", "keyword.control.import");
expect(1, "package", "keyword.control.import");
expect(1, "rust", "entity.name.namespace");
expect(2, "use", "keyword.control.use");
expect(2, "database", "entity.name.namespace");
expect(2, "dialect", "property");
expect(3, "moved", "keyword.control.moved");
expect(3, "->", "keyword.operator.arrow");
expect(5, "// A page and its action.", "comment.line.double-slash");
expect(6, "export", "storage.modifier.export");
expect(6, "nextjs.page", "storage.type.kind");
expect(6, "product_page", "entity.name.function.declaration");
expect(7, "route", "property");
expect(7, "/products/[id]", "string.quoted.double");
expect(9, '"""', "string.quoted.triple");
expect(10, "shop.cart", "variable.other.mention");
expect(10, "route", "variable.other.readwrite");
expect(14, "action", "storage.type.kind");
expect(14, "add_to_cart", "entity.name.function.declaration");
expect(15, "string", "support.type.primitive");
expect(15, "quantity", "property");
expect(16, "integer", "support.type.primitive");
expect(18, "expect", "entity.name.section");
expect(19, "adds_item", "property");
expect(23, "type", "storage.type.type");
expect(23, "Plan", "entity.name.type");
expect(23, "|", "keyword.operator");
expect(24, "let", "storage.type.let");
expect(24, "3", "constant.numeric");
expect(25, "extern", "storage.modifier.extern");
expect(25, "component", "storage.type.kind");
expect(25, "Form", "entity.name.function.declaration");
expect(28, "kind", "storage.type.kind");
expect(28, "page", "entity.name.type.class");
expect(30, "ref", "storage.modifier.ref");
expect(30, "action", "support.class.kind");
expect(30, "relation", "entity.name.function.decorator");
expect(33, "rule", "storage.type.kind");
expect(34, "select", "keyword.operator.word");
expect(34, "matches", "keyword.operator.word");

// Fenced blocks in Markdown (docs/spec.md writes them as ```spec).
const md = await registry.loadGrammar("markdown.sdlang.codeblock");
const r1 = md.tokenizeLine("```spec", vsctm.INITIAL);
const r2 = md.tokenizeLine("action a {", r1.ruleStack);
const scoped = r2.tokens.map((t) => t.scopes.join(" ")).join(" | ");
assert.ok(scoped.includes("meta.embedded.block.sdlang"), scoped);
assert.ok(scoped.includes("storage.type.kind.sdlang"), scoped);

console.log(`grammar: ${lines.length} lines tokenized, all checks passed`);
