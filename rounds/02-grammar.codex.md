I would keep the shared expression parser for values and types, but separate selectors, tighten declaration forms, and reject binding collisions instead of introducing shadowing.

The draft’s biggest risk is that “check by schema” becomes a catch-all for decisions the language has not made. It should determine whether an expression is valid in a field, never which symbol a name refers to or how tokens are grouped.

**A. Grammar defects**

**1. Distinguish syntax failures from examples that need migration.**

Several examples still parse under the draft but would fail the proposed schema or name checks. Those are different problems.

| Source | What happens now | Concrete fix |
|---|---|---|
| `to: @{login}` in `contributes` | Cannot parse: reference markers are only string content | `to: self`, or `to: login` |
| `path: template` | Cannot parse: `template` is reserved but has no expression production | Make `template` an ordinary builtin type name |
| `module auth { ... }` | Parses, despite the proposed unnamed module header | Change the example to `module { ... }`; reject named module headers |
| `created_at: timestamp @default(now)` | Parses; conflicts with the proposed annotation system | `created_at: timestamp = now` |
| `check: tested` | Parses as a reference, not an enum value | `check: "tested"` |
| Three `expect: [...]` examples | Parse; conflict with the proposed named-expectation schema | Use `expect: { redirects: "..." }` |
| Old `export kind page { route: ... }` | Parses; lacks the proposed schema/metadata separation | Move schema entries into `fields { ... }` |
| `NameTaken`, `RateLimited`, `layout`, `nav.item`, `typecheck`, `build` | Parse as paths; their bindings are not supplied | Declare them, supply them through a package manifest, or use strings where the schema means error codes |

The contribution example should become:

```intent
contributes: [
  nav.item {
    label: "Sign in"
    to: self
    when: "signed out"
  }
]
```

That still requires `nav.item` to resolve to a declared constructor with a field schema. Parsing `Path Record` does not define that constructor.

JSON records, terminal output, the directory tree and Mermaid examples are artifacts, not `.spec` programs; they should be labeled accordingly.

**2. Replace the generic declaration production.**

[The current declaration rule](/Users/mohebifar/dev/intentlang/GRAMMAR.ebnf:20) accepts all of these:

```intent
extern extern export export action foo {}
nextjs.page {}
stack named {}
action foo = 42
```

Schema checking should not have to discover that ordinary declarations require names, that `stack` does not, or that repeated modifiers are invalid.

Use dedicated statement forms. Here and below, `Gap ::= NL*`:

```ebnf
Statement   ::= Import
              | Use
              | Moved
              | ProjectDecl
              | StackDecl
              | ModuleDecl
              | 'export'? Exportable

Exportable  ::= LetDecl
              | TypeDef
              | KindDef
              | AnnotationDef
              | NamedDecl

ProjectDecl ::= 'project' Ident Block
StackDecl   ::= 'stack' Block
ModuleDecl  ::= 'module' Block

LetDecl     ::= 'let' Ident '=' Gap Expr
TypeDef     ::= 'type' Ident '=' Gap Expr
KindDef     ::= 'kind' Ident KindBody
AnnotationDef ::= 'annotation' Ident AnnotationBody

NamedDecl   ::= 'extern'? KindPath Ident Annotation* Block
```

This fixes modifier order and gives `=` a bounded role: constants, type aliases and schema defaults.

Separately enforce:

- One project and stack per project, only in the root module.
- One module header per directory.
- `use` only in the root module.
- Package definitions only in package sources.

Those are compilation-unit checks, since duplicates can occur across files.

**3. Entity fields collide with declaration metadata.**

This is a scope problem beyond REVIEW.md flaw 7:

```intent
entity Article {
  summary: string
  lifecycle: "draft" | "published"
}
```

Are those persisted fields, or the common declaration fields `summary` and `lifecycle`? Both interpretations are legitimate.

Use the same separation proposed for kind definitions:

```intent
entity Article {
  summary: "An authored article"
  lifecycle: "managed"

  fields {
    summary: string
    lifecycle: "draft" | "published" = "draft"
  }
}
```

Reserve direct body entries for kind-defined metadata. Put an entity’s user-defined schema in `fields`. Structural records such as `input: { email: email }` need no extra section because their interpretation is already fixed by `input`.

**4. Preserve newlines in the lexer.**

The current rule is incomplete. It drops newlines after `and` and `or`, but not after `where`, `not`, `matches`, `in`, `==` or `!=`:

```intent
applies_to: nextjs.page where
  route matches
    "/app/**"
```

That cannot parse, despite the stated continuation policy.

I would remove newline deletion entirely. Emit `NL` outside strings; let productions explicitly accept it after continuation tokens:

```ebnf
Field       ::= Ident ':' Gap Expr Default? Annotation*
Default     ::= '=' Gap Expr

Union       ::= Postfix ( '|' Gap Postfix )*
Negation    ::= 'not' Gap Negation | PredicateAtom
Comparison  ::= Access CompareOp Gap Constant
```

Allow newlines after openers, commas, colons, assignment/default operators and expression operators. Preserve newlines everywhere else.

Then these rules are clear:

```intent
plan: "free" |
  "team"                 // valid continuation

plan: "free"
  | "team"               // invalid: previous expression ended
```

Keep top-level statements newline-separated. Permit commas or newlines inside bodies, records, lists and annotation arguments. Choice 8 should state that scope explicitly.

Comments must stop **before** the newline, preserving the statement boundary. Comment recognition must be disabled inside strings.

**5. The draft is not directly LL(1), but most lookahead problems are easy to remove.**

`Member ::= Field | Section` has a shared `Ident` prefix. Left-factor it:

```ebnf
Member ::= Ident MemberRest
MemberRest ::= ':' Gap Expr Default? Annotation*
             | Block
```

Likewise, factor `export` at statement level as above.

Trailing separators also need care: `(Member (Sep Member)* Sep?)` requires looking beyond the separator to decide whether another member follows. A factored form is:

```ebnf
Block     ::= '{' Gap ( '}' | Member BlockTail )
BlockTail ::= '}' | Sep ( '}' | Member BlockTail )
Sep       ::= ',' Gap | NL+
```

Use analogous tails for lists and records.

`Path Record?` is manageable lookahead, not an inherent ambiguity: after finishing the path, `{` starts a constructor payload. But its meaning must be constrained:

- `nav.item { ... }`: constructor application.
- `login { ... }`: error if `login` resolves to an ordinary declaration.
- Namespace and kind bindings are not implicitly callable.

**6. Specify string scanning as a state machine.**

[The character productions](/Users/mohebifar/dev/intentlang/GRAMMAR.ebnf:80) do not implement their accompanying prose. Character-class subtraction cannot exclude multi-character openers reliably, and neither `@@{` nor `$${` has a production.

For example, `"@@{login}"` can consume the first `@` as text and then recognize `@{login}` as a reference—the opposite of the promised escape.

Specify ordered scanning inside a literal:

1. Recognize the applicable closing delimiter.
2. Recognize `@@{` and `$${` as literal opener escapes.
3. Recognize `@{Path}` and `${Ident}`.
4. Recognize quoted-string backslash escapes.
5. Consume ordinary text until the next special sequence.

Escaped output is never scanned again.

Additional rules:

- Prefer `"""` over `"` when opening a string.
- An ordinary `@`, email address, `|` or `//` inside a string is text.
- Malformed unescaped `@{...}` is an error; authors use `@@{` for literal examples.
- Markers are recognized even inside Markdown code fences. Otherwise parsing would depend on a second language.
- Reject raw LF and CR in line strings.
- Unicode escapes must denote Unicode scalar values, not surrogates or out-of-range integers.
- Give prose one delimiter escape, `\"""`; backslashes are otherwise literal.

I would also tighten interpolation:

```ebnf
Ref    ::= '@{' Path '}'
Interp ::= '${' Ident '}'
```

`${route}` binds a field declared by the surrounding kind definition. It does not search module imports, lets or capabilities. Dotted interpolation can wait until there is a demonstrated need.

**7. Make dedent mechanical.**

“Common leading indentation” leaves closing-line indentation, inline prose, blank lines and tabs unresolved.

I would define:

- Normalize CRLF to LF.
- A prose literal whose opener is immediately followed by LF is a block literal.
- Drop that opening LF.
- Require its closing delimiter on a line containing only indentation and the delimiter.
- Exclude the closing line when computing indentation.
- Remove the minimum leading-space count from nonblank content lines.
- Normalize blank content lines to empty lines.
- Preserve the final content newline.
- Reject tabs in block indentation.
- Inline triple-quoted literals receive no dedent.

That makes formatting and canonicalization agree.

**8. Shared parsing needs explicit interpretation rules.**

Retain one AST, but document the valid interpretations:

| AST | Value position | Type position |
|---|---|---|
| `"free"` | String value | Singleton string type |
| `[T]` | One-element list | List type |
| `[]` | Empty list | Error |
| `[A, B]` | Two-element list | Error; no implicit tuple type |
| `{ name: string }` | Record containing a type symbol, usually invalid | Structural record type |
| `A \| B` | Error | Union type |
| `ref T` | Error | Reference type |
| `T?` | Error | Optional type |

Defaults and annotations inside records are legal only when that record describes a schema. Reject them in ordinary data records.

Add `null`, and settle optionality: I suggest `T?` permits null and, for record fields, omission. Defaults apply to omission, not explicit null.

Most importantly, `now` must be a **symbolic initializer**. Compilation records “initialize at creation time”; it must never sample the clock while evaluating the spec.

---

**B. Scoping and reference resolution**

I recommend **no implicit shadowing and no resolution fallback based on expected type**.

**Binding construction**

Collect all files in a module before resolving any expression. Build one lexical binding table containing:

- Local named declarations, types, kinds, annotations and lets.
- Import aliases declared anywhere in that module.
- Project-wide capability bindings introduced by `use`.
- The core prelude: kinds, builtin types, constants and core annotations.

Rules:

- Duplicate bindings are errors, including declaration-versus-let and capability-versus-import collisions.
- Identical repeated imports may coalesce. This preserves the worked example’s repeated `import nextjs` across files in one directory.
- Imports belong to the module, not the file.
- Imports and lets do not flow into child modules.
- Inherited context does not import names.
- Capabilities are project-wide and cannot be rebound locally.
- Fields do not introduce lexical bindings.
- Core prelude names cannot be shadowed.

Add explicit import sources:

```intent
import module auth as auth_pages
import package "nextjs" as nextjs
```

The short form remains:

```intent
import auth
```

It succeeds only when exactly one local-module or package source matches. If both match, require an explicit source.

```ebnf
Import ::= 'import' ImportSource ( 'as' Ident )?
ImportSource ::= Path
               | 'module' Path
               | 'package' PlainLine
```

Package names are strings because scoped and hyphenated names are not identifier paths. Require an alias when no valid identifier can be derived.

**Resolution of `a.b.c`**

1. Resolve `a` once in the lexical binding table. An absent binding is an error.
2. Traverse each remaining segment according to the resolved object:
   - **Module namespace:** select an immediate child module or named declaration/let.
   - **Package or capability namespace:** select an exported manifest member.
   - **Constant record:** evaluate the immutable constant, then select its key.
   - **Anything else:** error; declarations are not records whose fields can be traversed.
3. Enforce visibility whenever traversal reaches a declaration or let outside the current module.
4. Validate the resulting symbol against the consuming field’s schema.
5. Never retry another interpretation if step 4 fails.

Thus, if `a` denotes an action, `a.b.c` does not fall back to a package called `a`.

Imports and capability aliases are not automatically re-exported as module members.

**Explicit roots**

Provide three reserved roots and one self-reference:

```intent
this.sign_in              // current module
root.auth.login           // absolute project module path
builtin.string            // core prelude
self                      // current named declaration
```

Immediate child directories are navigable through module namespaces; they do not automatically create unqualified lexical bindings. Reject a declaration or let sharing a name with an immediate child module, so namespace traversal remains unique.

`self.foo` is an error: self denotes a declaration reference, not access to its fields.

Use the same resolver for bare paths and `@{...}`. After resolution, a string mention must denote a valid graph target. A constant may alias a declaration reference; a constant string is not a mention target.

**Capability/module collision**

```intent
use "auth-provider" for auth
import module auth as auth_pages

action sign_in {
  requires: [auth.session]
  prompt: "Continue at @{auth_pages.login}."
}
```

`auth` is the capability; `auth_pages` is the module alias. `root.auth.login` is also unambiguous.

An unaliased `import auth` colliding with the capability is an error. Neither binding shadows the other.

**Forward references and cycles**

- Forward declaration and let references are legal.
- Constant-evaluation cycles are errors.
- Nonproductive type aliases such as `type A = A` are errors.
- Declaration-reference cycles remain legal as SPEC.md promises.

Package sources resolve against their own imports and dependencies, never the consuming project’s capability table. Consequently, `ui.component` in the package example needs an explicit package dependency.

Also, fetching a library’s documentation cannot create typed exports such as `authentication.User`. Those require a manifest, adapter or explicit extern definition.

Finally, `moved` operands need a separate `Address` category. They are canonical project addresses, not lexical references: the old address may no longer exist, and an import alias must not change its meaning.

---

**C. Annotations**

Declare annotations as typed package exports. Do not make unknown annotations silently accepted metadata.

For example:

```intent
export annotation index {
  targets: ["entity_field"]
  accepts: ["scalar"]
  projection: "public"

  parameters {
    method: "btree" | "hash" = "btree"
  }
}
```

A capability or package may export it:

```intent
fields {
  slug: string @database.index("btree")
}
```

The definition is declarative. `targets`, accepted type families and projection categories come from finite core-defined vocabularies. Packages cannot install arbitrary compiler callbacks through annotation declarations.

```ebnf
Annotation ::= '@' Path ( '(' Gap Arguments? ')' )?
Arguments  ::= Expr ( Sep Expr )* Sep?
```

Treat `parameters` as an **ordered schema section** if arguments remain positional. Its order must survive canonicalization. I would keep positional arguments initially rather than add another named-argument grammar.

Checking proceeds as follows:

1. Resolve the annotation path through the ordinary resolver.
2. Require an annotation definition.
3. Check the target category.
4. Check applicability to the annotated field’s type.
5. Check argument count, types and defaults.
6. Reject duplicate applications unless the definition explicitly permits repetition.
7. Include the application and definition hash in the declared projection.

Package generators and checks can consume this metadata. The compiler validates its declared shape; it does not invent enforcement behavior from the annotation’s name.

I would put these in core:

| Annotation | Meaning |
|---|---|
| `@id` | Entity identity field; initially one nonoptional scalar field |
| `@unique` | Entity-field uniqueness constraint |
| `@max_length(n)` | String-length constraint; clearer than overloaded `@max` |
| `@interface` | A kind-schema field participates in its public projection |
| `@deprecated(message)` | Diagnostic on references to a declaration |

Database index methods and framework-specific directives belong in packages. `@default` disappears; unknown uses should suggest `= expr`.

There is a case for declaration annotations:

```intent
export action old_sign_in @deprecated("Use sign_in") {
  ...
}
```

Allow them after the name and before the body, as in `NamedDecl` above. Require an explicitly declared declaration target. Keep ordinary settings such as `lifecycle` as fields; do not introduce synonymous annotations.

---

**D. The selector language**

The existing predicates cover the route example. The unrestricted `expr where predicate` leaves too much undefined: a string, record, declaration reference or type union can all appear to be selector sources.

Use an explicit selector expression:

```intent
applies_to: select [nextjs.page, nextjs.endpoint] where (
  field.route matches "/app/**" and
  meta.module != "marketing"
)
```

```ebnf
Expr          ::= Selector | Union

Selector      ::= 'select' SelectSource
                  ( 'where' Gap Predicate )?
SelectSource  ::= '*' | KindList
KindList      ::= '[' Gap KindPath ( Sep KindPath )* Sep? ']'

Predicate     ::= Conjunction ( 'or' Gap Conjunction )*
Conjunction   ::= Negation ( 'and' Gap Negation )*
Negation      ::= 'not' Gap Negation
                | '(' Gap Predicate Gap ')'
                | Test

Test          ::= 'exists' FieldRef
                | Access CompareOp Gap Constant

Access        ::= FieldRef | MetaRef
FieldRef      ::= 'field' '.' Ident ( '.' Ident )*
MetaRef       ::= 'meta' '.' ( 'module' | 'name' | 'kind' )

CompareOp     ::= '==' | '!=' | '<' | '<=' | '>' | '>='
                | 'matches' | 'in' | 'contains'
```

`Constant` is a literal, finite constant list, or path resolving to a compile-time constant/reference identity—not an arbitrary expression or another selector.

Separating `field` and `meta` avoids another collision: a declaration may legitimately define fields named `name`, `module` or `kind`.

The useful additions are:

```intent
exists field.layout
field.method in ["GET", "HEAD"]
field.tags contains "security"
field.requires contains authentication.session
field.timeout > 30
meta.kind == nextjs.page
```

Define the semantics narrowly:

- `in`: scalar membership in a finite list.
- `contains`: membership in a collection, not substring searching.
- Reference equality compares canonical target identities.
- Nested field access traverses local literal records only; it never follows graph references.
- Missing fields make atomic comparisons false, including `!=`.
- `exists` distinguishes omission from explicit null.
- Negation is ordinary Boolean negation; therefore `not missing_comparison` is true.
- An unknown field across every selected kind is an error.
- Operators must be valid for every selected kind in which the field exists. Conflicting field types require separate selectors.

Define `matches` as a full-string, case-sensitive glob with a small fixed syntax. For routes, let `*` match within a slash-delimited segment and `**` match zero or more whole segments. Explicitly make `/app/**` match `/app`. Exclude regexes, brace expansion and package-defined matching functions.

For attachment to remain cheap and deterministic:

1. Candidate declarations are project-managed instances of the selected kinds, within the rule’s module subtree.
2. Root rules therefore apply project-wide.
3. Evaluate against authored fields plus deterministic schema defaults, before rule context is attached.
4. Exclude inherited prompts, other rule results, evidence, output files, runtime state and clock values.
5. Forbid graph traversal, transitive dependencies, recursive selectors, callbacks, I/O and environment access.
6. Attach requirements cumulatively; rules cannot mutate the fields selectors inspect.
7. Rule order has no semantic effect.

Private descendants can receive inherited rules without becoming importable from the rule’s module. Selection does not grant reference or code-import rights.

I would make an empty match a warning, not the compile error currently listed in SPEC.md. Rules explicitly apply to declarations added later; an initially empty rule is useful.

---

**E. Versions in `stack`**

Use records. The grammar already has them, and they separate package/tool identity from a version requirement:

```intent
stack {
  language: { name: "typescript" }
  runtime: { name: "nodejs", version: "22" }
  framework: { name: "nextjs", version: "^15.0.0" }
  package_manager: { name: "pnpm" }
}
```

Give every slot the same closed shape:

```text
ToolRequirement {
  name: string
  version?: VersionRequirement
}
```

`VersionRequirement` is a validated string subtype with a specified range grammar. For v0, support exact versions, major/minor ranges, caret and tilde ranges; reject floating tags such as `"latest"`.

The source contains a requirement; the lock contains the exact resolution. Compilation reads the lock and performs no network resolution.

Do not split strings at `@`. That conflicts with scoped package names and hides structure inside strings.

Also distinguish the target framework version from the version of the spec package defining `nextjs.page`. Their compatibility is a package constraint; they are not automatically the same version.

No new EBNF is necessary for `stack` versions—only its field schema changes.

---

**F. Cuts and simplifications**

I would make these additional choices:

- **Keep string unions; remove `enum`.** Also remove it from the core-kind table and context-declaration list.
- **Keep identifier capability roles.** Schema-check the trailing `use` record against the provider’s options schema; it is not an arbitrary bag of configuration.
- **Remove declaration expansion templates from v0.** SPEC.md promises them without syntax or evaluation rules. Retain the `template` string type for package output patterns; describe declaration expansion as deferred.
- **Keep `ref` exclusively as a type operator.** Bare paths already represent graph references in value positions. Specify that `ref User?` means an optional reference.
- **Keep `|` exclusively for type unions.** Selector kind lists remove the need to overload it with set union.
- **Restrict section names by definition schema.** `Ident Block` may be convenient parsing machinery, but it must not silently create new sections.
- **Publish the core symbol table.** List builtin types and constants, including `uuid`, `timestamp`, `email`, `route_path`, `template` and `now`; identify anything intentionally left to packages.
- **Keep error codes and references distinct.** If `errors` means named codes, write `errors: ["NameTaken", "LimitReached"]`. If it means reusable error declarations, define those declarations. Never treat an unresolved path as an enum value.

The revised kind example would have this structure:

```intent
export kind page {
  fields {
    route: route_path @interface
    rendering: "static" | "server" | "client" = "server"
    layout: ref layout?
    uses: [ref ui.component]
    calls: [ref action]
  }

  outputs {
    page: "src/app/${route}/page.tsx"
  }

  checks: [typecheck, build]
}
```

Here `outputs` entries are template-typed by the core kind-definition schema. The package must still supply `layout`, `ui`, `typecheck` and `build`. It must also define how a route becomes a safe relative output path; string substitution alone does not establish that mapping.

**Changes I would make to `GRAMMAR.ebnf`, in priority order**

1. Replace newline deletion with preserved `NL` tokens and explicit continuation positions.
2. Specify string scanner modes, delimiter precedence, literal-marker escapes, malformed markers and exact prose dedent.
3. Replace generic declarations and repeated modifiers with dedicated statement forms.
4. Introduce entity `fields` sections to eliminate schema/metadata collisions.
5. Define lexical bindings, qualified roots, visibility, collision errors and explicit module/package imports.
6. Publish the expression interpretation matrix; add `null`, define optionality and restrict defaults to schema positions.
7. Give `now` symbolic default semantics without clock evaluation.
8. Replace postfix `where` with `select`; define field/meta namespaces and the bounded predicate grammar.
9. Add qualified annotation references and declarative annotation definitions, including target and argument checking.
10. Make kind-definition sections explicit and remove `template` from the reserved-word list.
11. Left-factor members, exported statements and trailing separators.
12. Separate canonical addresses used by `moved` from ordinary resolvable paths.
13. Document records for stack version requirements.
14. Update every legacy example and remove the undefined v0 promises for enums and declaration templates.