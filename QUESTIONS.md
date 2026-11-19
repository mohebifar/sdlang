# Open questions for the author

Matters of taste that change the language's look and feel. Each has a provisional default
in SPEC.md that the design does not depend on; changing any of them is a rename, not a
redesign.

| # | Question | Provisional default | Alternatives considered | Notes |
| --- | --- | --- | --- | --- |
| Q1 | What is the language called? File extension and CLI name? | Working name "intentlang"; files `.spec`; CLI `spec` | `.intent` / `intent`; a coined name | `spec` collides with many existing tools and with the generic word, which makes search and error messages ambiguous. A distinct name is recommended before any public release. |
| Q2 | Block style | Braces `{ }` with newline-or-comma separators | Indentation (YAML/Python style); `end` keywords | Braces make prose blocks and nesting unambiguous and keep the grammar newline-tolerant. |
| Q3 | Keyword for acceptance criteria | `expect` | `requires`, `accept`, `must` | `requires` is taken by the capability relation. |
| Q4 | Keyword for renames | `moved a -> b` | `renamed`, `was` (`login was sign_in`) | Mirrors Terraform's `moved` block, which users may know. |
| Q5 | Mention syntax inside prose | `@{path}` with `@@{` as the escape | `[[path]]` (wiki style), `{{path}}` | `[[...]]` reads well but collides with Markdown reference links in some renderers; `{{...}}` collides with template languages people paste into prompts. |
| Q6 | Prose string delimiter | `"""..."""` with indentation stripping | Backtick fences, heredocs | Prompts contain Markdown backticks, so backticks were rejected in v0. |
| Q7 | Enum-like values | String literals (`verify: "tested"`, `lifecycle: "frozen"`) | Bare identifiers resolved by schema (`verify: tested`) | Bare identifiers are shorter but make name resolution depend on the expected type, which the grammar round rejected. |
| Q8 | Selector keyword | `select <kinds> where <predicate>` | `match`, `every`, a `where`-only form | |
| Q9 | Verification shorthand names | `"tested"`, `"judged"`, `"accepted"` for `{tests,review}`, `{review}`, `{approval}` | `"test"`, `"review"`, `"approve"` | Past participles read as a status as well as a policy, which is mildly confusing; imperative verbs would avoid that. |
| Q10 | Entity schema section name | `fields { }` | `columns { }`, `schema { }` | `fields` is also the section name inside kind definitions, which keeps one concept for "the user-defined schema". |
