# macrokid_core

> Core abstractions and utilities for building Rust procedural macros

The foundation of the Macrokid framework, providing high-level abstractions that enable developers to write proc-macros without directly wrestling with syn/quote complexity.

## Features

- **Type Introspection (IR)**: Normalized representation of Rust types
- **Code Generation**: Trait-based code generation with combinators
- **Attribute Parsing**: Schema-based validation and parsing
- **Builder Patterns**: Fluent APIs for generating impl blocks and methods
- **Pattern DSL**: Type-safe pattern construction (feature-gated)
- **Derive Patterns**: Reusable derive macro patterns

## Quick Start

Add to your proc-macro crate:

```toml
[dependencies]
macrokid_core = { path = "../macrokid_core" }
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
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

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

## Core Modules

### Intermediate Representation (IR)

Located in `src/ir.rs`:

- `TypeSpec` - Normalized view of structs and enums
- `FieldKind` - Unified field representation (Named, Unnamed, Unit)
- `VariantSpec` - Enum variant information with attributes
- `FieldSpec` - Field metadata including type, visibility, and attributes

### Attribute Schema

Located in `src/common/attr_schema.rs`:

```rust
use macrokid_core::common::attr_schema::AttrSchema;

let schema = AttrSchema::new("my_attr")
    .req_str("name")
    .opt_int("count")
    .opt_bool("enabled")
    .opt_float("scale");

let parsed = schema.parse(&attrs)?;
```

Supported types: `str`, `int`, `bool`, `float`

### Code Generation

Located in `src/common/gen.rs`:

```rust
use macrokid_core::common::gen::{CodeGen, ResultCodeGen};

// Infallible generation
impl CodeGen for MyGenerator {
    fn gen_code(&self) -> TokenStream2 {
        quote! { /* generated code */ }
    }
}

// Fallible generation
impl ResultCodeGen for MyFallibleGenerator {
    fn gen_code(&self) -> syn::Result<TokenStream2> {
        Ok(quote! { /* generated code */ })
    }
}
```

### Builders

Located in `src/common/builders.rs`:

- `ImplBuilder` - Generate impl blocks with methods and associated items
- `MatchArmBuilder` - Construct match expressions

## Feature Flags

- `pattern_dsl` - Enable typed pattern construction API (`PatternSpec`)
- `log` - Enable `log::trace!` support for `#[trace]` macro

## Documentation

See the main [documentation](../docs/) for:
- [Architecture Overview](../docs/architecture/overview.md)
- [IR Guide](../docs/architecture/ir.md)
- [Code Generation Tutorial](../docs/guides/codegen-tutorial.md)
- [API Reference](../docs/reference/api.md)

## Examples

For usage examples, see:
- [examples/custom_derive](../examples/custom_derive/) - Custom derive macros
- [macrokid_graphics_derive](../macrokid_graphics_derive/) - Real-world derives

## Version Compatibility

- `syn = "2"`, `quote = "1"`, `proc-macro2 = "1"`
- MSRV: Rust 1.70+ (2021 edition)
- Uses modern `ParseNestedMeta` API (not deprecated `NestedMeta`)

## License

Part of the Macrokid project. See [LICENSE](../LICENSE) for details.
