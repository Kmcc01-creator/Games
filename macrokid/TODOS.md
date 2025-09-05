## Project TODOs and Roadmap

This document tracks the remaining phases after Phase 1 (semantic helpers) and captures a concise summary and concrete requirements for each.

**Phase 2 — Attribute Parsing Helpers**
- Summary: Extend `common::attrs` to support non-string attribute types and schema validation while preserving syn 2.x best practices.
- Requirements:
  - Add `attr_bool_value`, `attr_int_value`, and `has_flag` (marker attribute) helpers.
  - Add `validate_attrs(attrs, name, schema)` to enforce expected keys and value types; include precise spans in errors.
  - Keep backward compatibility; no changes to existing helpers’ behavior.
  - Update API docs and add small tests for each helper (trybuild or unit tests).

**Phase 3 — Diagnostics Utilities**
- Summary: Provide consistent, span-aware error helpers to improve macro error messages.
- Requirements:
  - Implement `err_on<T: Spanned>(node, msg) -> syn::Error` and `suggest_with_note<T: Spanned>(node, msg, note) -> syn::Error`.
  - Use throughout new helpers to standardize diagnostics.
  - Document usage patterns and examples in API docs.

**Phase 4 — ImplBuilder Enhancements**
- Summary: Expand `ImplBuilder` to support associated items and metadata.
- Requirements:
  - Add `add_assoc_type(name: Ident, ty: TokenStream2)` and `add_assoc_const(name: Ident, ty: TokenStream2, value: TokenStream2)`.
  - Add `with_docs(docs: &str)` and `with_attrs(attrs: TokenStream2)` to attach doc comments and arbitrary attributes to the impl block.
  - Ensure generics handling remains correct (`split_for_impl`).
  - Update API docs with examples.

**Phase 5 — Trace Attribute Options**
- Summary: Make `#[trace]` configurable without changing defaults.
- Requirements:
  - Parse nested options: `#[trace(prefix = "...", release = true|false, logger = "eprintln"|"log")]`.
  - If `logger = "log"`, emit `log::trace!` guarded behind an optional `log` feature; default remains `eprintln!`.
  - Add `cfg!(debug_assertions)` gate when `release = false` to disable in release builds without overhead.
  - Document behavior and provide examples; maintain existing semantics when no options are provided.

**Phase 6 — Match Validation Helpers (Heuristic)**
- Summary: Best-effort helpers to assist with match exhaustiveness and fallbacks.
- Requirements:
  - Provide `suggest_wildcard_if_non_exhaustive(enum_spec, builder) -> MatchArmBuilder` to append `_` arm with a diagnostic string when appropriate.
  - Document limitations (compiler remains the source of truth for exhaustiveness/unreachability).

**Phase 7 — Generics Utilities**
- Summary: Helpers to manipulate generics and where-clauses in codegen.
- Requirements:
  - Implement `add_trait_bounds(generics, bound_path)` to add trait bounds to type params.
  - Implement `push_where_predicate(generics, predicate_ts)` for ad-hoc where predicates.
  - Ensure idempotence and avoid duplicating bounds; add tests on simple generic types.

**Phase 8 — Enum Utilities**
- Summary: Convenience utilities for common enum patterns.
- Requirements:
  - `variant_arms(enum_spec, body_for)` → Vec<TokenStream2> for splicing.
  - `variant_names_const(enum_ident)` → emit `const VARIANTS: &[&str]` for any enum (not only bracket-style).
  - Document usage and performance considerations (compile-time only).

**Phase 9 — Function-like Parser Helpers**
- Summary: Reusable parsers for common macro syntaxes.
- Requirements:
  - `parse_ident_list` supporting comma-separated idents in `()` and `[]` forms.
  - `parse_kv_list` for `key = "value", ...` macro configs returning a small map or vector.
  - Add examples and tests; ensure syn 2.x idioms.

**Phase 10 — Typed Pattern DSL (Feature-Flagged)**
- Summary: Introduce a structured pattern API (e.g., `PatternSpec`, `FieldPatterns`) behind a feature flag.
- Requirements:
  - Define typed pattern structs/enums and a builder that lowers to tokens.
  - Support guards, alternates (`|`), ranges, slices, and `..` elision.
  - Provide migration notes and examples; do not alter current `MatchArmBuilder` behavior.

**Phase 11 — Arm Ordering and Optimization Hints**
- Summary: Non-semantic helpers to influence code layout/readability.
- Requirements:
  - Add `order_by(indices)` or `optimize_arm_order(frequency_hints)` to `MatchArmBuilder`.
  - Clarify that the compiler may reorder internally; intent is readability, not performance guarantees.

**Phase 12 — Examples and Tests**
- Summary: Demonstrate and validate new helpers.
- Requirements:
  - Add example derives for new helpers (attribute parsing, diagnostics, generics, enum utilities).
  - Add `trybuild` UI tests for error messages and `cargo expand` snippets in docs.
  - Keep examples minimal and focused; avoid runtime dependencies unless necessary.

**Phase 13 — Documentation Updates**
- Summary: Keep docs aligned with implementation.
- Requirements:
  - Update `API_REFERENCE.md`, `MATCH_ARM_BUILDER.md`, `README.md`, and `ONBOARDING.md` for each phase.
  - Include migration notes, especially when adding new public API (non-breaking) or feature flags.
  - Provide links between sections for discoverability.

---

Tracking: Mark each phase as completed when code, examples, tests, and docs are merged.
