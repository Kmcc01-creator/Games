# MatchArmBuilder: Technical Implementation and Project Impact

## Executive Summary

MatchArmBuilder is a lightweight helper in macrokid_core that makes it easy to assemble match expressions from reusable arm snippets. It favors flexibility by accepting raw token patterns and bodies, leaving exhaustiveness and type checks to the Rust compiler. This document explains the current implementation, how to use it effectively, and outlines an incremental roadmap aligned with the more ambitious DSL-style proposal you shared.

## Current Implementation

Location: macrokid_core/src/common/builders.rs

```rust
pub struct MatchArmBuilder {
    arms: Vec<TokenStream2>,
}

impl MatchArmBuilder {
    pub fn new() -> Self { Self { arms: Vec::new() } }

    /// Add a match arm by providing a pattern and a body expression.
    pub fn add_arm(mut self, pattern: TokenStream2, body: TokenStream2) -> Self {
        let arm = quote! { #pattern => #body };
        self.arms.push(arm);
        self
    }

    /// Build a full match expression given a scrutinee expression.
    pub fn build_match(self, scrutinee: TokenStream2) -> TokenStream2 {
        let arms = &self.arms;
        quote! {
            match #scrutinee {
                #( #arms ),*
            }
        }
    }

    /// Retrieve only the arms for embedding in an existing match.
    pub fn build_arms(self) -> Vec<TokenStream2> { self.arms }
}
```

Key properties:
- Pattern freedom: Patterns are raw tokens; anything valid after `match` is allowed (guards with `if`, alternates `|`, destructuring, refs, elision `..`).
- Body freedom: Bodies are expressions or blocks; all arms must unify to a single type (enforced by the compiler).
- No validation: The builder doesn’t enforce exhaustiveness or detect unreachable arms. The Rust compiler handles those checks.

## Usage Patterns

### Enum Display (simple per-variant arms)
```rust
let mut b = MatchArmBuilder::new();
for v in &enum_spec.variants {
    let vi = &v.ident;
    let name = vi.to_string();
    b = b.add_arm(
        quote! { Self::#vi { .. } },
        quote! { f.write_str(#name) }
    );
}
let body = b.build_match(quote! { self });
```

### Guards and Multi-Patterns
```rust
let b = MatchArmBuilder::new()
    // guard inline in the pattern
    .add_arm(quote! { Self::Val(x) if *x > 0 }, quote! { "positive" })
    // alternates with |
    .add_arm(quote! { Self::A | Self::B }, quote! { "simple" })
    // fallback
    .add_arm(quote! { _ }, quote! { "other" });

let expr = b.build_match(quote! { self });
```

### Destructuring Named and Tuple Fields
```rust
// Named
let b = MatchArmBuilder::new()
    .add_arm(
        quote! { Self::Point { x, y } },
        quote! { format!("({}, {})", x, y) }
    );

// Unnamed (tuple)
let b = b.add_arm(
    quote! { Self::Rgb(r, g, b) },
    quote! { format!("#{:02x}{:02x}{:02x}", r, g, b) }
);
```

### Splicing Arms into Existing Matches
```rust
let arms = MatchArmBuilder::new()
    .add_arm(quote! { Self::Ok(v) }, quote! { Some(v) })
    .add_arm(quote! { _ }, quote! { None })
    .build_arms();

let expr = quote! {
    match value {
        #( #arms ),*
    }
};
```

## Relationship to IR (TypeSpec, EnumSpec, FieldKind)

MatchArmBuilder complements the IR by turning discovered shape information into match arms:
- Enums: add an arm per `VariantSpec`, often ignoring fields with `{ .. }` when fields are irrelevant.
- Structs: match named or tuple fields selectively; `FieldSpec.ty` enables type-aware codegen (e.g., typed getters or validators).

Example (type-aware accessor for exposed fields):
```rust
let mut b = MatchArmBuilder::new();
if let FieldKind::Named(fields) = &struct_spec.fields {
    for f in fields {
        if has_attr(&f.attrs, "expose") {
            let fi = f.ident.as_ref().unwrap();
            b = b.add_arm(
                quote! { Self { #fi, .. } },
                quote! { return Some(#fi); }
            );
        }
    }
}
let body = b.add_arm(quote! { _ }, quote! { None }).build_match(quote! { self });
```

## Semantic Helpers (New)

Two helpers bridge IR and MatchArmBuilder for common cases:

```rust
use macrokid_core::patterns::{match_variants, match_fields};

// Build one arm per enum variant
let display_body = match_variants(&enum_spec, |v| {
    let vi = &v.ident;
    let name = vi.to_string();
    (quote! { Self::#vi { .. } }, quote! { f.write_str(#name) })
}).build_match(quote! { self });

// Build arms for named/tuple struct fields (skip unit)
let getter_body = match_fields(&struct_spec.fields, |f| {
    if let Some(ident) = &f.ident {
        Some((quote! { Self { #ident, .. } }, quote! { return Some(#ident) }))
    } else {
        None
    }
})
.add_wildcard(quote! { None })
.build_match(quote! { self });
```

## Differences vs. Proposed DSL

The proposal introduces a richer DSL (typed patterns, guards as first-class, validation, optimization hints). Our current implementation intentionally keeps the surface small and flexible. Key differences:

- Scrutinee handling:
  - Proposed: `on(scrutinee)` stored in builder; `build()` uses it.
  - Current: `build_match(scrutinee)` takes scrutinee at build time.

- Guards and wildcards:
  - Proposed: `add_arm_with_guard`, `add_wildcard` helpers.
  - Current: encode guards and `_` directly in the `pattern` token stream.

- Pattern modeling:
  - Proposed: `PatternSpec` enum to model patterns structurally.
  - Current: raw tokens; leverage `quote!` and compiler diagnostics for correctness.

- Validation and optimization:
  - Proposed: exhaustiveness checks, unreachable detection, ordering hints.
  - Current: rely on the Rust compiler to report non-exhaustive or unreachable arms.

## Roadmap (Incremental, Backward-Compatible)

Low-risk, high-value additions that keep existing API working:
- Convenience methods:
  - `add_wildcard(body)` → shorthand for `add_arm(quote! { _ }, body)`.
  - `add_guarded_arm(pattern_no_guard, guard, body)` → keeps guard separate for readability.
  - `add_multi_pattern(patterns, body)` → builds `A | B | C` automatically.

- Semantic helpers (optional, separate module):
  - `match_variants(&EnumSpec, mapper)` → build arms per variant.
  - `match_fields(&FieldKind, mapper)` → build arms per field.

- Validation (best effort):
  - `suggest_wildcard_if_non_exhaustive(expected_variants)` for enums (added as a heuristic helper that appends a wildcard arm using `unreachable!(..)`).
  - Documentation-driven guidance acknowledging compiler is the source of truth.

Larger feature (needs design iteration):
- Typed pattern DSL (`PatternSpec`) and guard API. This increases implementation complexity and surface area; we recommend prototyping behind a feature flag.

## Best Practices

- Decide ownership with scrutinee: `self` moves, `&self` borrows. Match ergonomics often remove the need for verbose `ref`.
- Prefer `{ .. }` elision to ignore fields you don’t use; it keeps patterns resilient to future changes.
- Add a final `_` arm when generating non-exhaustive matches intentionally. The compiler will enforce exhaustiveness otherwise.
- Keep arm bodies uniform in type to satisfy the compiler; wrap arms in blocks if you need local statements.

## Testing & Tooling

- UI tests with `trybuild` for patterns you expect to error (e.g., non-exhaustive generation when not desired).
- `cargo expand` to verify generated match expressions.
- Snapshot tests (e.g., `insta`) for token stream output if desired.

## Impact Assessment

- Stability: The current builder is simple and stable; proposed additions can be layered without breaking changes.
- Ergonomics: Convenience helpers and semantic wrappers would reduce boilerplate markedly for derive macros.
- Performance: Purely compile-time code generation; no runtime impact. Generated code is equivalent to handwritten matches.
