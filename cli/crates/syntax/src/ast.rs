//! The syntax tree. Every node carries the byte span of the source it came from
//! (offsets into the CRLF-normalised text). The tree serialises to JSON with
//! snake_case field names; enums carry a tag field naming the variant.

use serde::Serialize;

pub use crate::strings::StringLit;

/// A byte range `[start, end)` into the normalised source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// The smallest span covering both.
    pub fn to(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

/// A word in the source: an identifier, field name or path segment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Name {
    pub text: String,
    pub span: Span,
}

/// `Ident ( '.' Word )*`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Path {
    pub segments: Vec<Name>,
    pub span: Span,
}

impl Path {
    pub fn dotted(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// A whole `.sd` file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct File {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "statement", rename_all = "snake_case")]
pub enum Statement {
    Import(Import),
    Use(Use),
    Moved(Moved),
    Project(Project),
    Stack(Stack),
    Module(ModuleHeader),
    Decl(Decl),
}

impl Statement {
    pub fn span(&self) -> Span {
        match self {
            Statement::Import(s) => s.span,
            Statement::Use(s) => s.span,
            Statement::Moved(s) => s.span,
            Statement::Project(s) => s.span,
            Statement::Stack(s) => s.span,
            Statement::Module(s) => s.span,
            Statement::Decl(s) => s.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Import {
    pub source: ImportSource,
    pub alias: Option<Name>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ImportSource {
    /// `import nextjs`: a package or module by name.
    Path(Path),
    /// `import module auth`
    Module(Path),
    /// `import package "./spec-packages/acme"`
    Package(StringLit),
}

/// `use "drizzle" for database { ... }`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Use {
    pub library: StringLit,
    pub capability: Name,
    pub options: Option<Record>,
    pub span: Span,
}

/// `moved a.b -> a.c` or `moved module a -> b`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Moved {
    pub module: bool,
    pub from: Address,
    pub to: Address,
    pub span: Span,
}

/// A canonical address, never resolved through bindings: `auth.login` or
/// `auth.login#expect.redirects` (after `#`: collection, then key segments).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Address {
    pub segments: Vec<Name>,
    pub member: Option<Vec<Name>>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Project {
    pub name: Name,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stack {
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModuleHeader {
    pub body: Block,
    pub span: Span,
}

/// `'export'? Exportable`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decl {
    pub export: bool,
    pub item: Exportable,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "form", rename_all = "snake_case")]
pub enum Exportable {
    Let {
        name: Name,
        value: Expr,
    },
    Type {
        name: Name,
        value: Expr,
    },
    Kind {
        name: Name,
        body: Block,
    },
    Annotation {
        name: Name,
        body: Block,
    },
    /// `implements action { ... }`: how a package materialises a core kind.
    Implements {
        kind: Name,
        body: Block,
    },
    Named(NamedDecl),
}

/// `'extern'? Path Ident Annotation* Block`, e.g. `nextjs.page login { ... }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NamedDecl {
    #[serde(rename = "extern")]
    pub is_extern: bool,
    pub kind: Path,
    pub name: Name,
    pub annotations: Vec<Annotation>,
    pub body: Block,
}

/// `{ member* }`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Block {
    pub members: Vec<Member>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Member {
    pub name: Name,
    pub value: MemberValue,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "member", rename_all = "snake_case")]
pub enum MemberValue {
    /// `name: expr = default @annotation`
    Field(Field),
    /// `name { ... }`
    Section(Block),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Field {
    pub value: Expr,
    pub default: Option<Expr>,
    pub annotations: Vec<Annotation>,
}

/// `@path` or `@path(args)`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Annotation {
    pub name: Path,
    pub args: Option<Vec<Expr>>,
    pub span: Span,
}

/// One expression grammar for values and types; the field's schema decides the reading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "expr", rename_all = "snake_case")]
pub enum Expr {
    Selector(Box<Selector>),
    Union {
        variants: Vec<Expr>,
        span: Span,
    },
    Optional {
        inner: Box<Expr>,
        span: Span,
    },
    Ref {
        target: Box<Expr>,
        span: Span,
    },
    String(StringLit),
    Number {
        text: String,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Null {
        span: Span,
    },
    #[serde(rename = "self")]
    SelfRef {
        span: Span,
    },
    /// A reference, or with a record, a constructor record.
    Path {
        path: Path,
        record: Option<Record>,
        span: Span,
    },
    List {
        items: Vec<Expr>,
        span: Span,
    },
    Record(Record),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Selector(s) => s.span,
            Expr::String(s) => s.span,
            Expr::Record(r) => r.span,
            Expr::Union { span, .. }
            | Expr::Optional { span, .. }
            | Expr::Ref { span, .. }
            | Expr::Number { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Null { span }
            | Expr::SelfRef { span }
            | Expr::Path { span, .. }
            | Expr::List { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Record {
    pub fields: Vec<RecordField>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecordField {
    pub name: Name,
    pub field: Field,
    pub span: Span,
}

/// `select source where predicate`
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Selector {
    pub source: SelectSource,
    pub predicate: Option<Predicate>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "source", content = "kinds", rename_all = "snake_case")]
pub enum SelectSource {
    All,
    Kind(Path),
    Kinds(Vec<Path>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "predicate", rename_all = "snake_case")]
pub enum Predicate {
    Or {
        terms: Vec<Predicate>,
    },
    And {
        terms: Vec<Predicate>,
    },
    Not {
        inner: Box<Predicate>,
    },
    Exists {
        access: Access,
    },
    Compare {
        access: Access,
        op: CompareOp,
        operand: Operand,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "access", rename_all = "snake_case")]
pub enum Access {
    /// `meta.module`, `meta.name` or `meta.kind`
    Meta { field: Name },
    /// A field of the candidate, possibly nested.
    Field { path: Vec<Name> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Matches,
    In,
    Contains,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "operand", rename_all = "snake_case")]
pub enum Operand {
    String(StringLit),
    Number { text: String },
    Bool { value: bool },
    Null,
    Path(Path),
    List { items: Vec<Operand> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_cover_both() {
        assert_eq!(Span::new(3, 5).to(Span::new(1, 4)), Span::new(1, 5));
    }

    #[test]
    fn json_uses_snake_case_tags() {
        let expr = Expr::Bool {
            value: true,
            span: Span::new(0, 4),
        };
        let json = serde_json::to_string(&expr).unwrap();
        assert_eq!(
            json,
            r#"{"expr":"bool","value":true,"span":{"start":0,"end":4}}"#
        );
    }
}
