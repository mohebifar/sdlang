//! Recursive-descent parser from tokens to the syntax tree, mirroring docs/grammar.ebnf.
//!
//! Line breaks are significant: a production accepts one only where the grammar says
//! `Gap`, `NL+` or `Sep`. Parsing stops at the first error.

use std::fmt;

use crate::ast::*;
use crate::lexer::{self, is_keyword, Punct, Token, TokenKind};

/// A syntax error: what was expected, what was found, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn at(message: impl Into<String>, span: Span) -> Self {
        ParseError {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parses normalised source (see [`lexer::normalize`]) into a [`File`].
pub fn parse(src: &str) -> Result<File, ParseError> {
    let tokens = lexer::lex(src)?;
    Parser { tokens, pos: 0 }.file()
}

type PResult<T> = Result<T, ParseError>;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

const COMPARE_PUNCT: &[(Punct, CompareOp)] = &[
    (Punct::EqEq, CompareOp::Eq),
    (Punct::NotEq, CompareOp::Ne),
    (Punct::Lt, CompareOp::Lt),
    (Punct::Le, CompareOp::Le),
    (Punct::Gt, CompareOp::Gt),
    (Punct::Ge, CompareOp::Ge),
];

impl Parser {
    // ------------------------------------------------------------ helpers

    fn peek_at(&self, k: usize) -> &Token {
        let i = (self.pos + k).min(self.tokens.len() - 1);
        &self.tokens[i]
    }

    fn peek(&self) -> &Token {
        self.peek_at(0)
    }

    fn at_punct(&self, p: Punct) -> bool {
        self.at_punct_k(p, 0)
    }

    fn at_punct_k(&self, p: Punct, k: usize) -> bool {
        matches!(&self.peek_at(k).kind, TokenKind::Punct(q) if *q == p)
    }

    fn at_kw(&self, kw: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Word(w) if w == kw)
    }

    fn at_word_k(&self, k: usize) -> bool {
        matches!(&self.peek_at(k).kind, TokenKind::Word(_))
    }

    fn at_newline(&self) -> bool {
        self.at_newline_k(0)
    }

    fn at_newline_k(&self, k: usize) -> bool {
        matches!(self.peek_at(k).kind, TokenKind::Newline)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn take(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn err<T>(&self, expected: &str) -> PResult<T> {
        let tok = self.peek();
        Err(ParseError::at(
            format!("expected {expected}, found {}", tok.kind.describe()),
            tok.span,
        ))
    }

    fn expect_punct(&mut self, p: Punct) -> PResult<Span> {
        if self.at_punct(p) {
            Ok(self.take().span)
        } else {
            self.err(&format!("'{}'", p.as_str()))
        }
    }

    fn expect_kw(&mut self, kw: &str) -> PResult<Span> {
        if self.at_kw(kw) {
            Ok(self.take().span)
        } else {
            self.err(&format!("'{kw}'"))
        }
    }

    /// `Gap ::= NL*`
    fn gap(&mut self) {
        while self.at_newline() {
            self.take();
        }
    }

    /// Any word, keywords included.
    fn word(&mut self) -> PResult<Name> {
        match &self.peek().kind {
            TokenKind::Word(w) => {
                let text = w.clone();
                let span = self.take().span;
                Ok(Name { text, span })
            }
            _ => self.err("a name"),
        }
    }

    /// A word that is not a keyword.
    fn ident(&mut self) -> PResult<Name> {
        match &self.peek().kind {
            TokenKind::Word(w) if !is_keyword(w) => self.word(),
            _ => self.err("an identifier (not a keyword)"),
        }
    }

    fn line_string(&mut self) -> PResult<StringLit> {
        match &self.peek().kind {
            TokenKind::Str(s) if s.kind == crate::strings::StringKind::Line => {
                let s = s.clone();
                self.take();
                Ok(s)
            }
            _ => self.err("a line string"),
        }
    }

    /// `Path ::= Ident ( '.' Word )*`
    fn path(&mut self) -> PResult<Path> {
        let first = self.ident()?;
        let mut span = first.span;
        let mut segments = vec![first];
        while self.at_punct(Punct::Dot) && self.at_word_k(1) {
            self.take();
            let seg = self.word()?;
            span = span.to(seg.span);
            segments.push(seg);
        }
        Ok(Path { segments, span })
    }

    /// After an item in a bracketed sequence: consume a `Sep`; true if the list closed.
    fn sep_or_close(&mut self, close: Punct) -> PResult<Option<Span>> {
        let mut had_sep = false;
        if self.at_punct(Punct::Comma) {
            self.take();
            self.gap();
            had_sep = true;
        } else if self.at_newline() {
            self.gap();
            had_sep = true;
        }
        if self.at_punct(close) {
            return Ok(Some(self.take().span));
        }
        if !had_sep {
            return self.err(&format!("',' or a line break or '{}'", close.as_str()));
        }
        Ok(None)
    }

    /// `open Gap ( item ( Sep item )* Sep? )? Gap close`; returns the items and the span
    /// from the opener to the closer.
    fn seq<T>(
        &mut self,
        open: Punct,
        close: Punct,
        mut item: impl FnMut(&mut Self) -> PResult<T>,
    ) -> PResult<(Vec<T>, Span)> {
        let start = self.expect_punct(open)?;
        self.gap();
        let mut items = Vec::new();
        if self.at_punct(close) {
            let end = self.take().span;
            return Ok((items, start.to(end)));
        }
        loop {
            items.push(item(self)?);
            if let Some(end) = self.sep_or_close(close)? {
                return Ok((items, start.to(end)));
            }
        }
    }

    // ------------------------------------------------------------ file

    fn file(mut self) -> PResult<File> {
        let mut statements = Vec::new();
        self.gap();
        while !self.at_eof() {
            statements.push(self.statement()?);
            if self.at_eof() {
                break;
            }
            if !self.at_newline() {
                return self.err("a line break between statements");
            }
            self.gap();
        }
        let end = self.peek().span.end;
        Ok(File {
            statements,
            span: Span::new(0, end),
        })
    }

    fn statement(&mut self) -> PResult<Statement> {
        let start = self.peek().span;
        if self.at_kw("import") {
            self.take();
            let source = if self.at_kw("module") {
                self.take();
                ImportSource::Module(self.path()?)
            } else if self.at_kw("package") {
                self.take();
                ImportSource::Package(self.line_string()?)
            } else {
                ImportSource::Path(self.path()?)
            };
            let alias = if self.at_kw("as") {
                self.take();
                Some(self.ident()?)
            } else {
                None
            };
            let span = start.to(self.prev_end());
            return Ok(Statement::Import(Import {
                source,
                alias,
                span,
            }));
        }
        if self.at_kw("use") {
            self.take();
            let library = self.line_string()?;
            self.expect_kw("for")?;
            let capability = self.ident()?;
            let options = if self.at_punct(Punct::LBrace) {
                Some(self.record()?)
            } else {
                None
            };
            let span = start.to(self.prev_end());
            return Ok(Statement::Use(Use {
                library,
                capability,
                options,
                span,
            }));
        }
        if self.at_kw("moved") {
            self.take();
            let module = if self.at_kw("module") {
                self.take();
                true
            } else {
                false
            };
            let from = self.address()?;
            self.expect_punct(Punct::Arrow)?;
            let to = self.address()?;
            let span = start.to(to.span);
            return Ok(Statement::Moved(Moved {
                module,
                from,
                to,
                span,
            }));
        }
        if self.at_kw("project") {
            self.take();
            let name = self.ident()?;
            let body = self.block()?;
            let span = start.to(body.span);
            return Ok(Statement::Project(Project { name, body, span }));
        }
        if self.at_kw("package") {
            self.take();
            let name = self.ident()?;
            let body = self.block()?;
            let span = start.to(body.span);
            return Ok(Statement::Package(PackageHeader { name, body, span }));
        }
        if self.at_kw("stack") {
            self.take();
            let body = self.block()?;
            let span = start.to(body.span);
            return Ok(Statement::Stack(Stack { body, span }));
        }
        if self.at_kw("module") {
            self.take();
            let body = self.block()?;
            let span = start.to(body.span);
            return Ok(Statement::Module(ModuleHeader { body, span }));
        }
        let export = if self.at_kw("export") {
            self.take();
            true
        } else {
            false
        };
        let item = self.exportable()?;
        let span = start.to(self.prev_end());
        Ok(Statement::Decl(Decl { export, item, span }))
    }

    /// The span of the last consumed token.
    fn prev_end(&self) -> Span {
        let i = self.pos.saturating_sub(1);
        self.tokens[i].span
    }

    fn address(&mut self) -> PResult<Address> {
        let first = self.word()?;
        let mut span = first.span;
        let mut segments = vec![first];
        while self.at_punct(Punct::Dot) {
            self.take();
            let seg = self.word()?;
            span = span.to(seg.span);
            segments.push(seg);
        }
        let mut member = None;
        if self.at_punct(Punct::Hash) {
            self.take();
            let mut path = vec![self.word()?];
            self.expect_punct(Punct::Dot)?;
            path.push(self.word()?);
            while self.at_punct(Punct::Dot) {
                self.take();
                path.push(self.word()?);
            }
            span = span.to(path.last().expect("non-empty").span);
            member = Some(path);
        }
        Ok(Address {
            segments,
            member,
            span,
        })
    }

    fn exportable(&mut self) -> PResult<Exportable> {
        if self.at_kw("let") || self.at_kw("type") {
            let is_let = self.at_kw("let");
            self.take();
            let name = self.ident()?;
            self.expect_punct(Punct::Eq)?;
            self.gap();
            let value = self.expr()?;
            return Ok(if is_let {
                Exportable::Let { name, value }
            } else {
                Exportable::Type { name, value }
            });
        }
        if self.at_kw("kind") || self.at_kw("annotation") {
            let is_kind = self.at_kw("kind");
            self.take();
            let name = self.ident()?;
            let body = self.block()?;
            return Ok(if is_kind {
                Exportable::Kind { name, body }
            } else {
                Exportable::Annotation { name, body }
            });
        }
        if self.at_kw("implements") {
            self.take();
            let kind = self.word()?;
            let body = self.block()?;
            return Ok(Exportable::Implements { kind, body });
        }
        self.named_decl().map(Exportable::Named)
    }

    fn named_decl(&mut self) -> PResult<NamedDecl> {
        let is_extern = if self.at_kw("extern") {
            self.take();
            true
        } else {
            false
        };
        let kind = self.path()?;
        let name = self.ident()?;
        let mut annotations = Vec::new();
        while self.at_punct(Punct::At) {
            annotations.push(self.annotation()?);
        }
        let body = self.block()?;
        Ok(NamedDecl {
            is_extern,
            kind,
            name,
            annotations,
            body,
        })
    }

    // ------------------------------------------------------------ bodies

    fn block(&mut self) -> PResult<Block> {
        let (members, span) = self.seq(Punct::LBrace, Punct::RBrace, Self::member)?;
        Ok(Block { members, span })
    }

    fn member(&mut self) -> PResult<Member> {
        let name = self.word()?;
        if self.at_punct(Punct::Colon) {
            self.take();
            self.gap();
            let field = self.field_rest()?;
            let span = name.span.to(self.prev_end());
            Ok(Member {
                name,
                value: MemberValue::Field(field),
                span,
            })
        } else if self.at_punct(Punct::LBrace) {
            let body = self.block()?;
            let span = name.span.to(body.span);
            Ok(Member {
                name,
                value: MemberValue::Section(body),
                span,
            })
        } else {
            self.err("':' or '{' after a member name")
        }
    }

    fn field_rest(&mut self) -> PResult<Field> {
        let value = self.expr()?;
        let default = if self.at_punct(Punct::Eq) {
            self.take();
            self.gap();
            Some(self.expr()?)
        } else {
            None
        };
        let mut annotations = Vec::new();
        while self.at_punct(Punct::At) {
            annotations.push(self.annotation()?);
        }
        Ok(Field {
            value,
            default,
            annotations,
        })
    }

    fn annotation(&mut self) -> PResult<Annotation> {
        let start = self.expect_punct(Punct::At)?;
        let name = self.path()?;
        let mut span = start.to(name.span);
        let args = if self.at_punct(Punct::LParen) {
            let (args, s) = self.seq(Punct::LParen, Punct::RParen, Self::expr)?;
            span = span.to(s);
            Some(args)
        } else {
            None
        };
        Ok(Annotation { name, args, span })
    }

    fn record(&mut self) -> PResult<Record> {
        let (fields, span) = self.seq(Punct::LBrace, Punct::RBrace, |p| {
            let name = p.word()?;
            p.expect_punct(Punct::Colon)?;
            p.gap();
            let field = p.field_rest()?;
            let span = name.span.to(p.prev_end());
            Ok(RecordField { name, field, span })
        })?;
        Ok(Record { fields, span })
    }

    // ------------------------------------------------------------ expressions

    fn expr(&mut self) -> PResult<Expr> {
        if self.at_kw("select") {
            Ok(Expr::Selector(Box::new(self.selector()?)))
        } else {
            self.union()
        }
    }

    fn union(&mut self) -> PResult<Expr> {
        let first = self.postfix()?;
        if !self.at_punct(Punct::Pipe) {
            return Ok(first);
        }
        let mut span = first.span();
        let mut variants = vec![first];
        while self.at_punct(Punct::Pipe) {
            self.take();
            self.gap();
            let next = self.postfix()?;
            span = span.to(next.span());
            variants.push(next);
        }
        Ok(Expr::Union { variants, span })
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let mut expr = if self.at_kw("ref") {
            let start = self.take().span;
            let target = self.primary()?;
            let span = start.to(target.span());
            Expr::Ref {
                target: Box::new(target),
                span,
            }
        } else {
            self.primary()?
        };
        if self.at_punct(Punct::Question) {
            let q = self.take().span;
            let span = expr.span().to(q);
            expr = Expr::Optional {
                inner: Box::new(expr),
                span,
            };
        }
        Ok(expr)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Str(s) => {
                self.take();
                Ok(Expr::String(s.clone()))
            }
            TokenKind::Number(n) => {
                self.take();
                Ok(Expr::Number {
                    text: n.clone(),
                    span: tok.span,
                })
            }
            TokenKind::Word(w) if w == "true" || w == "false" => {
                self.take();
                Ok(Expr::Bool {
                    value: w == "true",
                    span: tok.span,
                })
            }
            TokenKind::Word(w) if w == "null" => {
                self.take();
                Ok(Expr::Null { span: tok.span })
            }
            TokenKind::Word(w) if w == "self" => {
                self.take();
                Ok(Expr::SelfRef { span: tok.span })
            }
            TokenKind::Word(_) => {
                let path = self.path()?;
                let record = if self.at_punct(Punct::LBrace) {
                    Some(self.record()?)
                } else {
                    None
                };
                let span = record.as_ref().map_or(path.span, |r| path.span.to(r.span));
                Ok(Expr::Path { path, record, span })
            }
            TokenKind::Punct(Punct::LBracket) => {
                let (items, span) = self.seq(Punct::LBracket, Punct::RBracket, Self::expr)?;
                Ok(Expr::List { items, span })
            }
            TokenKind::Punct(Punct::LBrace) => Ok(Expr::Record(self.record()?)),
            TokenKind::Punct(Punct::LParen) => {
                self.take();
                self.gap();
                let inner = self.expr()?;
                self.gap();
                self.expect_punct(Punct::RParen)?;
                Ok(inner)
            }
            _ => self.err("an expression"),
        }
    }

    // ------------------------------------------------------------ selectors

    fn selector(&mut self) -> PResult<Selector> {
        let start = self.expect_kw("select")?;
        let source = if self.at_punct(Punct::Star) {
            self.take();
            SelectSource::All
        } else if self.at_punct(Punct::LBracket) {
            let mut k = 1;
            while self.at_newline_k(k) {
                k += 1;
            }
            if self.at_punct_k(Punct::RBracket, k) {
                return self.err("a kind (a selector kind list may not be empty)");
            }
            let (paths, _) = self.seq(Punct::LBracket, Punct::RBracket, Self::path)?;
            SelectSource::Kinds(paths)
        } else {
            SelectSource::Kind(self.path()?)
        };
        let predicate = if self.at_kw("where") {
            self.take();
            self.gap();
            Some(self.predicate()?)
        } else {
            None
        };
        let span = start.to(self.prev_end());
        Ok(Selector {
            source,
            predicate,
            span,
        })
    }

    fn predicate(&mut self) -> PResult<Predicate> {
        let first = self.conjunction()?;
        if !self.at_kw("or") {
            return Ok(first);
        }
        let mut terms = vec![first];
        while self.at_kw("or") {
            self.take();
            self.gap();
            terms.push(self.conjunction()?);
        }
        Ok(Predicate::Or { terms })
    }

    fn conjunction(&mut self) -> PResult<Predicate> {
        let first = self.negation()?;
        if !self.at_kw("and") {
            return Ok(first);
        }
        let mut terms = vec![first];
        while self.at_kw("and") {
            self.take();
            self.gap();
            terms.push(self.negation()?);
        }
        Ok(Predicate::And { terms })
    }

    fn negation(&mut self) -> PResult<Predicate> {
        if self.at_kw("not") {
            self.take();
            self.gap();
            return Ok(Predicate::Not {
                inner: Box::new(self.negation()?),
            });
        }
        if self.at_punct(Punct::LParen) {
            self.take();
            self.gap();
            let inner = self.predicate()?;
            self.gap();
            self.expect_punct(Punct::RParen)?;
            return Ok(inner);
        }
        self.test()
    }

    fn test(&mut self) -> PResult<Predicate> {
        if self.at_kw("exists") {
            self.take();
            return Ok(Predicate::Exists {
                access: self.access()?,
            });
        }
        let access = self.access()?;
        let op = match &self.peek().kind {
            TokenKind::Punct(p) => COMPARE_PUNCT
                .iter()
                .find(|(q, _)| q == p)
                .map(|(_, op)| *op),
            TokenKind::Word(w) => match w.as_str() {
                "matches" => Some(CompareOp::Matches),
                "in" => Some(CompareOp::In),
                "contains" => Some(CompareOp::Contains),
                _ => None,
            },
            _ => None,
        };
        let Some(op) = op else {
            return self.err("a comparison operator");
        };
        self.take();
        self.gap();
        let operand = self.operand()?;
        Ok(Predicate::Compare {
            access,
            op,
            operand,
        })
    }

    fn access(&mut self) -> PResult<Access> {
        if self.at_kw("meta") {
            self.take();
            self.expect_punct(Punct::Dot)?;
            match &self.peek().kind {
                TokenKind::Word(w) if matches!(w.as_str(), "module" | "name" | "kind") => {
                    let field = self.word()?;
                    return Ok(Access::Meta { field });
                }
                _ => return self.err("meta.module, meta.name or meta.kind"),
            }
        }
        let mut path = vec![self.word()?];
        while self.at_punct(Punct::Dot) {
            self.take();
            path.push(self.word()?);
        }
        Ok(Access::Field { path })
    }

    fn operand(&mut self) -> PResult<Operand> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Str(s) => {
                self.take();
                Ok(Operand::String(s.clone()))
            }
            TokenKind::Number(n) => {
                self.take();
                Ok(Operand::Number { text: n.clone() })
            }
            TokenKind::Word(w) if w == "true" || w == "false" => {
                self.take();
                Ok(Operand::Bool { value: w == "true" })
            }
            TokenKind::Word(w) if w == "null" => {
                self.take();
                Ok(Operand::Null)
            }
            TokenKind::Punct(Punct::LBracket) => {
                let (items, _) = self.seq(Punct::LBracket, Punct::RBracket, Self::operand)?;
                Ok(Operand::List { items })
            }
            _ => Ok(Operand::Path(self.path()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(src: &str) -> File {
        parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
    }

    fn fails(src: &str) -> ParseError {
        parse(src).expect_err(src)
    }

    #[test]
    fn statements_need_line_breaks() {
        assert_eq!(ok("import a\nimport b\n").statements.len(), 2);
        let e = fails("import a import b\n");
        assert!(e.message.contains("line break between statements"), "{e}");
    }

    #[test]
    fn expressions_break_only_after_operators() {
        ok("type Plan = \"free\" |\n  \"team\"\n");
        fails("type Plan = \"free\"\n  | \"team\"\n");
        ok("action a {\n  input: {\n    a: string,\n    b: string\n  }\n}\n");
    }

    #[test]
    fn keywords_only_rejected_where_identifiers_required() {
        ok("ui.component Button {\n  type: \"button\"\n  module { kind: \"x\" }\n}\n");
        fails("entity type {\n}\n");
    }

    #[test]
    fn first_error_reports_span_and_found() {
        let e = fails("action a {\n  input: { a: string b: string }\n}\n");
        assert!(e.message.contains("found 'b'"), "{e}");
        assert_eq!(e.span, Span::new(32, 33));
    }

    #[test]
    fn tree_shape() {
        let file =
            ok("export nextjs.page login @unique {\n  calls: [sign_in]\n  route: \"/login\"\n}\n");
        let Statement::Decl(decl) = &file.statements[0] else {
            panic!("not a decl")
        };
        assert!(decl.export);
        let Exportable::Named(named) = &decl.item else {
            panic!("not named")
        };
        assert_eq!(named.kind.dotted(), "nextjs.page");
        assert_eq!(named.name.text, "login");
        assert_eq!(named.annotations.len(), 1);
        assert_eq!(named.body.members.len(), 2);
    }

    #[test]
    fn selectors() {
        let file = ok("rule r {\n  applies_to: select [nextjs.page, nextjs.endpoint] where\n    route matches\n      \"/app/**\" and not exists layout\n}\n");
        assert_eq!(file.statements.len(), 1);
        fails("rule r {\n  applies_to: select []\n}\n");
        fails("rule r {\n  applies_to: select * where meta.path == \"x\"\n}\n");
    }
}
