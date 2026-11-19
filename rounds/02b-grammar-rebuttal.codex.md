**1. Concede: bare candidate fields preserve the distinction.**

`route` and `meta.module` have distinct meanings under the editor’s rule. Fields named `name`, `module`, or `kind` cannot collide with metadata:

```intent
select nextjs.page where module != "marketing"
select nextjs.page where meta.module != "marketing"
```

The first tests the candidate’s authored field; the second tests its module address. Reserving `meta` as a candidate field name makes that separation complete. My `field.` prefix was unnecessary.

Refine “a bare identifier in a predicate is always a field” to **an unqualified access operand denotes a candidate field**. My proposed grammar also allows reference constants on the right:

```intent
requires contains authentication.session
```

Here `requires` is candidate access; `authentication.session` resolves through ordinary lexical bindings as a constant reference identity. Keep those grammar positions distinct, with no resolution fallback.

**2. Concede: remove `this`, `root`, and `builtin`.**

Given the binding rules I proposed, I see no necessary case for them.

- `this.sign_in` adds nothing when `sign_in` cannot be shadowed and fields introduce no lexical bindings.
- `builtin.string` adds nothing when the prelude’s `string` cannot be shadowed.
- `root.auth.login` supplies an alternative route around explicit imports. Requiring `import module auth as auth_pages` preserves access while making the dependency visible.

One accompanying change is necessary: my proposal allowed immediate child modules to be traversed through module namespaces without giving them unqualified bindings. Require an import for those children too; removing `this` must not leave an implicit exception or an inaccessible namespace.

Keep `self` as the current named declaration reference, including inside constructor records, with `self.foo` rejected. Keep `moved` operands as canonical addresses that neither resolve through aliases nor grant expression access.

This fits SPEC.md’s use of visibility to bound agent context.

**3. Refine: accept contextual names, and specify their token handling.**

There is no field-versus-statement ambiguity in my proposed body grammar: bodies contain fields and sections, while dedicated statements occur at file scope.

```ebnf
Member     ::= Word MemberRest
MemberRest ::= ':' Gap Expr Default? Annotation*
             | Block

Path       ::= Ident ( '.' Word )*
```

`Word` includes keyword spellings; `Ident` excludes words reserved at that head position. Thus:

```intent
type: "button"       // field
module { ... }      // section inside a body
settings.type       // path member
meta.kind           // selector metadata access
```

A section still needs schema authorization. Accepting its spelling does not permit arbitrary sections. Likewise, `meta.kind` retains its dedicated metadata interpretation.

The lexer must preserve keyword spellings so the parser can accept them as `Word`; it cannot discard them from all name productions globally.

There is one additional interaction with proposal 1: **bare predicate accesses also need a contextual-name rule**. Otherwise a field named `null`, `not`, or `exists` becomes legal to declare but unavailable through the advertised bare access syntax.

These can be distinguished using the following token:

```intent
not == true          // comparison against the field named "not"
not enabled == true  // Boolean negation
exists == true       // comparison against the field named "exists"
exists layout        // presence test
```

That requires lookahead or left-factoring beyond my original predicate sketch, but does not create two valid parses. I accept keyword field names; I would update the predicate productions explicitly rather than leave their accessibility implicit.