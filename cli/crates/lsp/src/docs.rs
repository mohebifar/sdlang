//! Short documentation for the language's fixed vocabulary, shown on hover and in
//! completion. Each entry paraphrases docs/spec.md; the spec is the authority.

pub fn keyword(word: &str) -> Option<&'static str> {
    Some(match word {
        "import" => "Brings a package (`import nextjs`), a module (`import module auth`) or a local package (`import package \"./dir\" as name`) into scope.",
        "export" => "Makes the declaration visible to other modules.",
        "extern" => "Existing code the spec refers to but does not manage. Accepts `source` and `summary`.",
        "use" => "Binds a library to a capability: `use \"drizzle\" for database { ... }`.",
        "for" => "Names the capability a `use` binds.",
        "as" => "Gives an import a local name.",
        "moved" => "Asserts that two addresses denote one continuing thing, so a rename keeps its history: `moved auth.login -> auth.sign_in`.",
        "let" => "A named value: `let name = expr`.",
        "type" => "A shared value type: `type Plan = \"free\" | \"team\"`.",
        "kind" => "Defines a kind (in a package): its fields, outputs, checks and conventions.",
        "annotation" => "Defines an annotation (in a package).",
        "implements" => "How a package materialises a core kind such as `action` or `lib`.",
        "project" => "The project's name and project-wide context (`summary`, `prompt`, `verify`).",
        "stack" => "Language, runtime, framework and package manager; owns toolchain files.",
        "module" => "Header for a directory of the spec: `summary`, `prompt`, `skills`.",
        "package" => "A package's header, alone in its `package.sd`: the import name, `entry`, `skills` and `provides`. In `import package \"./dir\"`: a local package directory.",
        "ref" => "A reference type: `ref action` is a reference to an action.",
        "self" => "The declaration being defined.",
        "select" => "A selector: `select action where meta.module == \"auth\"`.",
        "where" => "Starts a selector's predicate.",
        "and" | "or" | "not" => "Combines selector predicates.",
        "exists" => "In a selector: the field is present.",
        "meta" => "In a selector: `meta.module`, `meta.name` or `meta.kind` of a candidate.",
        "matches" | "in" | "contains" => "A selector comparison.",
        "true" | "false" => "A boolean.",
        "null" => "No value.",
        _ => return None,
    })
}

pub fn core_kind(name: &str) -> Option<&'static str> {
    Some(match name {
        "entity" => "A persisted data type: typed fields, relations, constraints. Managed.",
        "action" => "A callable contract, independent of transport: `input`, `output`, `errors`, `uses`, `requires`, `reads`, `writes`. Managed.",
        "lib" => "Shared code that belongs to no page or action: helpers, hooks, shared types. Fields `exports`, `path`. Managed.",
        "rule" => "A cross-cutting constraint: `applies_to` (a selector), `prompt`, `expect`. Context.",
        "environment" => "A deployment target the CLI observes. Context.",
        "check" => "A check a package defines and the CLI runs.",
        "generator" => "A deterministic generator a package defines.",
        "observer" => "Reads the state of an environment.",
        "binding" => "How a package binds a library to a capability.",
        _ => return None,
    })
}

pub fn builtin(name: &str) -> Option<&'static str> {
    Some(match name {
        "string" => "Text.",
        "number" => "A number.",
        "integer" => "A whole number.",
        "boolean" => "`true` or `false`.",
        "uuid" => "A UUID string.",
        "email" => "An email address.",
        "url" => "A URL.",
        "timestamp" => "A point in time.",
        "date" => "A calendar date.",
        "json" => "Any JSON value.",
        "route_path" => "A route such as `\"/products/[id]\"`.",
        "template" => "A string with `${field}` interpolations.",
        "secret" => "A value read from the environment, never written into the spec.",
        "catalog" => "Any catalog declaration, for reference-typed kind fields.",
        "now" => "A symbolic initializer: at creation time.",
        _ => return None,
    })
}

pub fn core_annotation(name: &str) -> Option<&'static str> {
    Some(match name {
        "id" => "`@id` (entity field): the identity field; one non-optional scalar.",
        "unique" => "`@unique` (entity field): a uniqueness constraint.",
        "max_length" => "`@max_length(n)` (string field): a length limit.",
        "interface" => "`@interface` (kind field): the field joins the public projection, so dependents are re-checked when it changes.",
        "relation" => "`@relation(\"name\")` (kind field): the relation of edges from this reference field.",
        "deprecated" => "`@deprecated(\"message\")` (declaration): a warning on every reference.",
        _ => return None,
    })
}

/// Fields and sections that core kinds share or define.
pub fn field(name: &str) -> Option<&'static str> {
    Some(match name {
        "summary" => "One line saying what this is. Part of the public projection.",
        "prompt" => "Normative requirements in prose. Agents must meet them and reviewers check them.",
        "hint" => "Non-normative guidance. Creates no obligation; the compiler warns when it reads as a requirement.",
        "uses" => "Components, catalog entries or libs this depends on. Relation `uses`: orders work, consumes the target's contract.",
        "skills" => "Package skills rendered into this declaration's work orders.",
        "lifecycle" => "`\"frozen\"` keeps the declaration's code as it is; `\"protected\"` blocks changes.",
        "expect" => "Requirements, one per key. Each must be covered by a test case tagged `[case:<key>]` or by review.",
        "errors" => "Named error cases with their messages. Addressable as `decl#errors.<key>`.",
        "input" => "The action's input type. Interface: callers are updated when it changes incompatibly.",
        "output" => "The action's output type. Interface.",
        "requires" => "Capabilities this needs, such as `authentication.session`. Relation `requires`.",
        "reads" => "Entities this reads. Relation `reads`: observes their behaviour.",
        "writes" => "Entities this writes. Relation `writes`.",
        "calls" => "Actions this calls. Relation `calls`: consumes the target's signature and observes its behaviour.",
        "exports" => "The names a lib exports. Interface.",
        "path" => "Where the lib's code lives.",
        "applies_to" => "A selector naming the declarations a rule constrains.",
        "verify" => "The project's default verification policy.",
        "verify_scope" => "`\"all\"` runs the whole test suite on every transaction.",
        "source" => "Where an extern's code lives.",
        "route" => "The page's route.",
        "fields" => "An entity's fields, or a kind definition's field schema.",
        _ => return None,
    })
}
