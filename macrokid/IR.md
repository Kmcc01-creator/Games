# IR (Intermediate Representation): Status and Roadmap

## Executive Summary

Macrokid’s IR (TypeSpec/EnumSpec/StructSpec/FieldSpec) provides a normalized, syn 2.x‑aligned view of derive inputs. It is the core substrate that our framework builds upon for attribute parsing, code generation builders, and semantic match helpers. The IR is sufficient for a large class of macros today, but there are targeted areas where additional helpers and data will unlock more advanced use cases.

## What We Have Today

- Container: `TypeSpec { ident, generics, attrs, kind }` with `TypeKind::{Struct, Enum}`.
- Structs: `StructSpec { fields: FieldKind }` where `FieldKind` is `Named/Unnamed/Unit`.
- Enums: `EnumSpec { variants: Vec<VariantSpec> }` with `VariantSpec { ident, attrs, fields }`.
- Fields: `FieldSpec { ident, index, attrs, ty, span }` — includes `syn::Type` for type‑aware generation.

Strengths:
- Clean, normalized shape that avoids low‑level syn details.
- Type info on fields enables typed getters/validation and smarter codegen.
- Good spans on fields; syn 2.x idioms across parsing.

## Gaps and Opportunities

- Visibility not captured (type/field). Helpful for re‑emitting items or conditional public APIs.
- Enum discriminants not captured (e.g., `A = 1`). Needed for value‑aware macros.
- Container/variant spans missing; field spans exist. Helpful for precise diagnostics beyond fields.
- `#[repr(...)]` info not normalized; FFI and layout‑sensitive macros benefit from this.

## Roadmap

### IR‑A (Non‑Breaking Helpers)
- Traversal helpers: convenience accessors and counts.
- Type inspection utilities: `is_option`, `is_vec`, `unwrap_result`, etc.
- Repr helper: parse `#[repr(...)]` into a normalized form.

### IR‑B (Light Extensions; Breaking)
- Add visibility to `TypeSpec`/`FieldSpec`.
- Add spans to `TypeSpec`/`VariantSpec`.
- Add `discriminant: Option<syn::Expr>` to `VariantSpec`.
- Provide accessor methods and migration notes to reduce positional destructuring.

### IR‑C (Ergonomics)
- Query API (iterators over filtered fields/variants).
- Meta view combining parsed repr/attrs into ready‑to‑use decisions.

## Decision

Proceed with IR‑A now (non‑breaking helpers). Plan IR‑B changes with a minor version bump (pre‑1.0) and document migration. Re‑evaluate IR‑C based on real usage after IR‑A lands.

