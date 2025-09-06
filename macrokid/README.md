# Macrokid

> A user-friendly framework for writing Rust proc-macros without directly touching syn/quote

Macrokid provides high-level abstractions and reusable components for common macro patterns, making procedural macro development accessible to developers who want to focus on logic rather than parsing complexity.

## 🎯 Project Mission

**Enable developers to write powerful Rust proc-macros using high-level abstractions instead of wrestling with syn/quote directly.**

## 📁 Project Structure

```
macrokid/
├── macrokid/              # Main proc-macro crate (user-facing)
├── macrokid_core/         # Framework abstractions and utilities
│   ├── ir.rs              # Type introspection and normalization
│   ├── attr/              # Attribute macro helpers
│   ├── function/          # Function-like macro helpers
│   └── common/            # Shared utilities (attrs, builders)
├── examples/
│   └── custom_derive/     # Example: Custom derive macros
└── example/               # Usage demonstrations
macrokid_graphics/         # Experimental graphics runtime
macrokid_graphics_derive/  # Experimental graphics derives (resources, buffers, pipelines)
macrokid_clang_exec/       # Exec-based Clang PoC for C/C++ headers
examples/graphics_demo/    # Demo using graphics derives + Clang PoC build script
```

## 🚀 Quick Start

### Using Macrokid Macros

Add macrokid to your `Cargo.toml`:

```toml
[dependencies]
macrokid = { path = "path/to/macrokid" }
```

Use the built-in macros:

```rust
use macrokid::{make_enum, trace};

// Function-like macro: Auto-generate enum with traits
make_enum!(Status: Active, Inactive, Pending);

// Attribute macro: Add function tracing (configurable)
#[trace(prefix = "[app]", release = true, logger = "eprintln")] // all args optional
fn process_data() -> Result<(), &'static str> {
    // Your logic here
    Ok(())
}
```

### Building Custom Macros

Create a proc-macro crate using macrokid_core:

```toml
[dependencies]
macrokid_core = { path = "path/to/macrokid_core" }
proc-macro2 = "1"
quote = "1" 
syn = { version = "2", features = ["full"] }
```

Example custom derive:

```rust
use macrokid_core::{
    ir::{TypeSpec, TypeKind},
    builders::ImplBuilder,
    attrs::attr_string_value,
};

#[proc_macro_derive(MyTrait, attributes(my_attr))]
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    
    // Parse into normalized IR
    let spec = TypeSpec::from_derive_input(input).unwrap();
    
    // Generate implementation using builder
    let impl_block = ImplBuilder::new(spec.ident, spec.generics)
        .implement_trait(quote! { MyTrait })
        .add_method(quote! {
            fn my_method(&self) -> String {
                "Hello from macro!".to_string()
            }
        })
        .build();
    
    impl_block.into()
}
```

## 🏗️ Architecture

### Core Framework (`macrokid_core`)

**Intermediate Representation (IR)**
- `TypeSpec` - Normalized view of Rust types (structs, enums)
- `FieldKind` - Unified field representation (Named, Unnamed, Unit)
- `VariantSpec` - Enum variant information with attributes

**High-Level Utilities**
- `ImplBuilder` - Fluent API for generating impl blocks
- `MatchArmBuilder` - Helper for match expressions
- Attribute parsing with `attr_string_value()`, `attr_bool_value()`, `attr_int_value()`, `has_flag()`, `parse_nested_attrs()`, `validate_attrs()`

**Module Organization**
- `ir.rs` - Type introspection and normalization
- `attr/` - Attribute macro helpers (e.g., `trace.rs`)
- `function/` - Function-like macro helpers (e.g., `make_enum.rs`)
- `common/` - Shared utilities (`attrs.rs`, `builders.rs`)

### Main Crate (`macrokid`)

Provides ready-to-use macros built with the framework:
- `#[trace]` - Function execution timing
- `make_enum!()` - Enum generation with derived traits

### Examples (`examples/custom_derive`)

Demonstrates advanced framework usage:
- `#[derive(Display)]` with `#[display("name")]` attributes
- `#[derive(DebugVerbose)]` with field-level `#[skip]` support

## 🧪 Experiments

We’re actively exploring graphics-focused DSLs and cross-language tooling:

- `macrokid_graphics` + `macrokid_graphics_derive`: ResourceBinding, BufferLayout, GraphicsPipeline derives.
- `macrokid_clang_exec`: Exec-based Clang integration to analyze/generate from C/C++ headers.
- `examples/graphics_demo`: Shows derives in action and emits C/C++ IR when `CLANG_EXEC_DEMO=1`.

See `EXPERIMENTS.md` for current status, usage, and roadmap. These APIs are experimental and may change.

## 🔧 Technical Details

### Modern syn 2.x API Integration

- Uses `ParseNestedMeta` for attribute parsing (replaces deprecated `NestedMeta`)
- Leverages `attr.parse_nested_meta()` for robust nested attribute handling
- Proper error spans and contextual error messages

### Version Compatibility

- `syn = "2"`, `quote = "1"`, `proc-macro2 = "1"`.
- MSRV: Rust 1.70+ (2021 edition).
- The core APIs avoid deprecated syn 1.x items (e.g., `NestedMeta`, `Attribute::parse_meta`).

### Common Migration Pitfalls (syn 1 → 2)

- Attribute parsing:
  - syn 1.x: `attr.parse_meta()?` and `NestedMeta`
  - syn 2.x: `attr.parse_nested_meta(|meta| { ... })` and `ParseNestedMeta`
- Bare string lists like `#[attr("a", "b")]` must use `meta.input.parse::<LitStr>()?` inside the closure.
- Name-value pairs like `#[attr(key = "val")]` use `meta.value()?.parse::<LitStr>()?`.
- Prefer `attr.path().is_ident("name")` over matching fields directly; `Attribute`’s internal layout changed.

Macrokid implements these patterns internally so consumers don’t have to deal with syn specifics.

### Type Support

- Types: structs and enums are supported via `TypeSpec`; unions are not supported.
- Fields: named, unnamed (tuple), and unit forms are normalized via `FieldKind`.
- Generics: generic params, lifetimes, and where-clauses are preserved (`split_for_impl`).
- Attributes: type-, variant-, and field-level attributes are exposed for parsing.
- Function-like macros: current `make_enum!` and bracket-syntax helper accept simple `Ident` lists.

Additional considerations:
- `#[trace]` works for both sync and async functions; timing is measured around the full function body.
  - Async-aware: the timer starts when the async body first runs and stops after completion; elapsed time includes `.await` periods.
- `common::attrs` currently parses string literals; parsing numbers/bools requires extending helpers.
- IR now includes field types (`FieldSpec.ty`).

### IR Change: Field Types in `FieldSpec`

- `FieldSpec` now includes a `ty: syn::Type` for each field.
- This enables type-aware macro generation (e.g., typed getters, validation).
- Impact: This is an API addition that may break code destructuring `FieldSpec` by fields; prefer using field access by name rather than positional patterns.

### IR-B: Visibility, Spans, Discriminants

- `TypeSpec`: `vis: syn::Visibility`, `span: proc_macro2::Span`.
- `VariantSpec`: `span: Span`, `discriminant: Option<syn::Expr>`.
- `FieldSpec`: `vis: syn::Visibility`.
- These additions improve diagnostics and enable more advanced macros. Update any positional destructuring to named fields.

### Feature Flags

- `pattern_dsl`: Enables a typed pattern builder API (`PatternSpec`) for constructing match patterns structurally.
- `log`: Enables `log::trace!` output for `#[trace(logger = "log")]` (falls back to `eprintln!` when disabled).

#### Pattern DSL Usage (feature: `pattern_dsl`)

```toml
[dependencies]
macrokid_core = { path = "path/to/macrokid_core", features = ["pattern_dsl"] }
```

```rust
use macrokid_core::pattern_dsl::{PatternSpec as P, StructFields};
use quote::ToTokens;
use syn::parse_quote;

// Build a pattern: Self::Point { x, .. } guarded by x > 0
let pat = P::Struct { path: parse_quote!(Self::Point), fields: StructFields {
    named: vec![(parse_quote!(x), P::Ident(parse_quote!(x)))],
    rest: true,
}};
let guarded = pat.with_guard(parse_quote!(x > 0));
let ts = guarded.into_tokens(); // usable with MatchArmBuilder
```

### Key Design Decisions

1. **Separation of Concerns**
   - Thin proc-macro entry points in main crate
   - Rich abstractions in core framework
   - Examples separate from core functionality

2. **Developer Experience First**
   - No syn/quote knowledge required for common patterns
   - Builder patterns for intuitive code generation
   - Rich attribute support out-of-the-box

3. **Extensible Architecture**
   - Plugin-style approach for new macro types
   - Reusable components across macro categories
   - Framework evolution without breaking changes

### Supported Macro Types

| Type | Status | Example | Features |
|------|--------|---------|----------|
| Attribute | ✅ | `#[trace]` | Function wrapping, custom parameters |
| Derive | 🧪 | `#[derive(Display)]` | Complex attribute parsing, field introspection |
| Function-like | ✅ | `make_enum!()` | Custom syntax, multiple output traits |

## 🎓 Developer Onboarding

### Prerequisites

- Rust 2021 edition knowledge
- Basic understanding of procedural macros
- Familiarity with derive macros preferred

### Learning Path

1. **Explore Examples** - Run `cargo run --bin example` to see working demos
2. **Study Custom Derives** - Examine `examples/custom_derive/src/lib.rs` 
3. **Build Your First Macro** - Start with a simple derive using `ImplBuilder`
4. **Advanced Features** - Explore nested attributes and IR manipulation

### Common Patterns

**Basic Derive Macro:**
```rust
let spec = TypeSpec::from_derive_input(input)?;
match spec.kind {
    TypeKind::Struct(s) => { /* handle struct */ },
    TypeKind::Enum(e) => { /* handle enum */ },
}
```

**Attribute Parsing:**
```rust
// Simple: #[attr("value")]
let value = attr_string_value(&attrs, "attr");

// Complex: #[attr(key = "value", other = "data")]
let nested = parse_nested_attrs(&attrs, "attr")?;
```

**Code Generation:**
```rust
ImplBuilder::new(type_name, generics)
    .implement_trait(quote! { std::fmt::Display })
    .add_method(quote! { fn fmt(&self, f: &mut Formatter) -> Result { ... } })
    .build()
```

## 🧪 Testing and Examples

### Running Examples

```bash
# Basic usage demonstration
cargo run --bin example

# Build all examples
cargo build

# Test custom derive functionality  
cargo test -p custom_derive
```

### Example Output

```
Color from str: Green
Mode: Fast
Mode (custom): SLOW
Struct display: Point2D
Config (verbose debug): CustomConfig { name: "MyApp", port: 8080 }
work returned 999
[macrokid::trace] work took 5.079µs
```

## 🚧 Current Status

### ✅ Completed Features

- Modern syn 2.x API integration
- IR with field types; IR-B metadata (visibility, spans, discriminants)
- Builder patterns for code generation (methods, assoc items, impl attrs)
- Attribute parsing helpers (string/bool/int, schema validation)
- Diagnostics utilities (span-aware errors and notes)
- Typed pattern DSL (feature-flagged)
- Trace options (prefix/release/logger with optional `log` feature)
- Working examples for all macro types and new helpers

### 🔄 In Progress

- Additional derive macro examples (using pattern DSL, advanced attributes)
- Match validation helpers (extended heuristics)
- CI and trybuild UI tests

### 📋 Planned Features

- Bracket-style macro syntax (`macro[syntax]`)
- Plugin system for custom macro types
- IDE integration helpers
- Comprehensive test suite / CI
- Performance benchmarking

## 💡 Contributing

The framework is designed to be extensible. Common contribution areas:

1. **New Builder Patterns** - Add helpers for common code generation patterns
2. **Attribute Parsers** - Extend support for complex attribute syntaxes  
3. **Example Macros** - Demonstrate advanced framework usage
4. **Documentation** - Improve onboarding and API docs
5. **Performance** - Optimize compilation times and memory usage

## 📚 Additional Resources

- [Rust Procedural Macros Book](https://doc.rust-lang.org/reference/procedural-macros.html)
- [syn Crate Documentation](https://docs.rs/syn/latest/syn/)
- [quote Crate Documentation](https://docs.rs/quote/latest/quote/)
- [proc-macro2 Crate Documentation](https://docs.rs/proc-macro2/latest/proc_macro2/)
- [MatchArmBuilder Guide](MATCH_ARM_BUILDER.md)
- [IR Overview and Roadmap](IR.md)

---

**Macrokid** - Making proc-macro development accessible to everyone 🚀

## 🔔 Breaking Changes

- IR: `FieldSpec` now includes `ty: syn::Type`. Code that destructures `FieldSpec` positionally may fail to compile; update patterns to include `ty` or use field names. This enables type-aware macro generation across the framework.
- IR-B: `TypeSpec` includes `vis: Visibility` and `span: Span`; `VariantSpec` includes `span: Span` and `discriminant: Option<Expr>`; `FieldSpec` includes `vis: Visibility`. If destructuring these structs positionally, update your patterns; using field names is forward-compatible.
