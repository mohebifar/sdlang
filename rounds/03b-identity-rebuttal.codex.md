1. **Concede: remove package and capability addresses from `moved`.**

   I conflated identities needed in the evaluated graph with addresses projects need to migrate.

   Package catalog continuity belongs to package-authored migration data. Changing an import alias preserves the canonical package symbol without a move; switching packages does not establish continuity between their exports.

   A capability binding’s name is sufficient identity for v0. Changing its provider or options changes consumed inputs and requires compatibility assessment. Renaming the binding can appear as removal plus addition in consumers’ context records, producing conservative invalidation. A capability move could someday suppress unnecessary work for a pure binding rename, but no current requirement makes it necessary.

   Keep `moved` limited to declaration addresses, schema-declared member paths, and explicit module moves. Canonical package symbols can remain in graph JSON and state without becoming surface-language move operands.

2. **Refine: withdraw the manifest; require recoverable applied effects.**

   Nothing requires a separate record for an output-free declaration that has never affected managed code. A rule with no applications has no applied baseline to preserve. When it later matches something, the planner evaluates its current definition.

   For existing consumers, per-target records can handle removal, rename and invalidation: compare old and current context keys, normalize addresses through moves, and reconcile requirement applications by `(target, requirement)`. Retained move statements supply continuity; git supplies authored history.

   There is one concrete limitation: **hashes identify changes but cannot explain or classify them**. Suppose an action consumes a shared type, then that type changes. The old hash alone cannot distinguish an optional-field addition from a required-field removal. Likewise, it cannot produce an old/new context diff for a work order.

   This needs either relevant canonical effect snapshots in the consumer’s record, or a reliable locator from that record to the exact applied spec revision and locked package inputs. Git history suffices when that revision is recorded and recoverable. Using the latest committed spec is insufficient after partial applies: different targets may have accepted different context revisions.

   If those old inputs are unavailable, conservative invalidation remains possible, but precise compatibility classification and historical diffs are unavailable. That is a baseline-data requirement, not an argument for a shared manifest.

3. **Concede, with one defaulting rule: shorthand creates no identity problem.**

   The existing `Member` production already admits both forms. Schema validation should lower both into the same canonical member before computing projections or hashes:

   ```spec
   slug_unique: "two workspaces never share a slug"
   ```

   is equivalent to a block containing that `text`, with the same effective verification policy. Both have identity `create_workspace#expect.slug_unique`. Switching forms alone must preserve hashes, test bindings and evidence validity.

   Require collection-declared primary text fields, reject unsupported scalar values, and reject duplicate keys across field and section forms.

   The subtlety is an error’s default wire code during a key rename:

   ```spec
   moved create_workspace#errors.name_taken -> create_workspace#errors.name_unavailable
   ```

   With omitted codes, the old effective code is `"name_taken"` and the new one is `"name_unavailable"`. Identity continues, but the public contract changes. Preserve the old default in the baseline comparison; do not recompute it from the destination key and erase the change.

   To rename only the identity key while preserving the wire contract, write `code: "name_taken"` explicitly at the destination. This is ordinary defaulting behavior, not a reason to require long-form members everywhere.