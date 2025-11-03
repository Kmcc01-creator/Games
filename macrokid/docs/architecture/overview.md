# Macrokid Architecture

This document provides a deep dive into the technical architecture, design decisions, and implementation details of the Macrokid framework.

## 🏗️ High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Applications                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Derive    │  │ Attribute   │  │   Function-like     │  │
│  │   Macros    │  │   Macros    │  │     Macros          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      macrokid                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   #[trace]  │  │ make_enum!  │  │   Future Macros     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    macrokid_core                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │     IR      │  │  Builders   │  │      Attrs          │  │
│  │ TypeSpec    │  │ ImplBuilder │  │ attr_string_value   │  │
│  │ FieldKind   │  │MatchArm     │  │ parse_nested_attrs  │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   syn + quote + proc-macro2                 │
└─────────────────────────────────────────────────────────────┘
```

## 🎯 Design Principles

### 1. **Abstraction Over Implementation**
- Users work with high-level concepts (`TypeSpec`, `ImplBuilder`)
- syn/quote complexity is hidden behind clean APIs
- Framework handles edge cases and error scenarios

### 2. **Composability and Reusability**
- Small, focused modules that can be combined
- Builder patterns for flexible code generation
- Shared utilities across macro types

### 3. **Developer Experience First**
- Intuitive APIs that match mental models
- Rich error messages with proper spans
- Extensive documentation and examples

### 4. **Modern Rust Best Practices**
- syn 2.x API integration
- Edition 2021 features
- Zero-cost abstractions where possible

### Type Support Overview

- Types: structs and enums are first-class; unions are not supported.
- Fields: named, unnamed (tuple), and unit forms are normalized via `FieldKind`.
- Generics: impl generation preserves generics, lifetimes, and where-clauses.
- Attributes: exposed across type/variant/field with modern syn 2.x parsing.
- Field types and metadata: `FieldSpec` includes `ty: syn::Type` and `vis: Visibility`. `TypeSpec` includes `vis: Visibility` and `span: Span`. `VariantSpec` includes `span: Span` and `discriminant: Option<syn::Expr>`.

## 📦 Crate Structure

### `macrokid_core` - The Framework Foundation

#### `ir.rs` - Intermediate Representation

The IR module provides a normalized, high-level view of Rust types that abstracts away syn's complexity:

```rust
// High-level type representation
pub struct TypeSpec {
    pub ident: Ident,           // Type name
    pub generics: Generics,     // Generic parameters
    pub attrs: Vec<Attribute>,  // Type-level attributes  
    pub kind: TypeKind,         // Struct or Enum
}

pub enum TypeKind {
    Struct(StructSpec),
    Enum(EnumSpec),
}

// Unified field representation
pub enum FieldKind {
    Named(Vec<FieldSpec>),    // struct { field: Type }
    Unnamed(Vec<FieldSpec>),  // struct(Type, Type)
    Unit,                     // struct;
}
```

**Key Benefits:**
- **Normalization**: All types follow consistent patterns
- **Simplification**: Complex syn AST reduced to essential information
- **Extensibility**: Easy to add new type information as needed (e.g., `FieldSpec.ty` now records field types)

#### `common/builders.rs` - Code Generation Helpers

Provides fluent APIs for generating common Rust constructs:

```rust
// Fluent impl block generation
ImplBuilder::new(type_name, generics)
    .implement_trait(quote! { std::fmt::Display })
    .add_method(quote! {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Implementation
        }
    })
    .build()

// Match arm generation
MatchArmBuilder::new()
    .add_arm(quote! { Variant::A }, quote! { "A" })
    .add_arm(quote! { Variant::B }, quote! { "B" })
    .build_match(quote! { self })
```

**Design Rationale:**
- **Readability**: Generated code structure is clear from builder calls
- **Consistency**: All impl blocks follow same patterns
- **Error Prevention**: Builder validates structure before generation

#### `common/attrs.rs` - Attribute Parsing

Modern syn 2.x attribute parsing utilities:

```rust
// Simple attribute values
attr_string_value(&attrs, "display")  // #[display("Custom")]

// Complex nested attributes  
parse_nested_attrs(&attrs, "config")  // #[config(key = "value", flag)]

// Attribute existence
has_attr(&attrs, "skip")             // #[skip]
```

**Modern syn 2.x Integration:**
- Uses `ParseNestedMeta` for proper error spans
- Leverages `attr.parse_nested_meta()` for robustness
- Handles both simple and complex attribute patterns

#### Module Organization

```
macrokid_core/src/
├── lib.rs                 # Public API exports
├── ir.rs                  # Type introspection and IR
├── common/
│   ├── mod.rs            # Common utilities module
│   ├── attrs.rs          # Attribute parsing utilities
│   └── builders.rs       # Code generation builders
├── attr/
│   ├── mod.rs            # Attribute macro helpers
│   └── trace.rs          # Example: function tracing
└── function/
    ├── mod.rs            # Function-like macro helpers
    ├── make_enum.rs      # Example: enum generation
    └── bracket_enum.rs   # Future: bracket syntax support
```

#### Async-Safe Tracing

The `#[trace]` attribute wraps the original function body and measures wall-clock time using `Instant`:
- For sync functions, it times execution of the entire function body.
- For async functions, the timer starts when the async body first runs and stops when it completes, spanning `.await` points.
- A nested expression ensures logging executes even when the original body returns early.

### `macrokid` - User-Facing Proc-Macros

Thin wrapper crate providing ready-to-use macros:

```rust
use syn::{parse_macro_input, ItemFn};
// Attribute macro entry point
#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(item as ItemFn);
    // Parse options into TraceConfig (prefix/release/logger) and delegate
    let cfg = /* build macrokid_core::attr::trace::TraceConfig from attr */;
    macrokid_core::attr::trace::expand_trace(func, cfg).into()
}

// Function-like macro entry point
#[proc_macro]
pub fn make_enum(input: TokenStream) -> TokenStream {
    let parsed_input: macrokid_core::function::make_enum::MakeEnumInput =
        parse_macro_input!(input as macrokid_core::function::make_enum::MakeEnumInput);
    
    // Delegate to framework
    macrokid_core::function::make_enum::expand_make_enum(parsed_input).into()
}
```

**Design Benefits:**
- **Clean API**: Users import simple, focused macros
- **Framework Evolution**: Core can evolve without breaking user code
- **Easy Extension**: New macros added with minimal boilerplate

### `examples/custom_derive` - Framework Demonstration

Shows how to build sophisticated macros using the framework:

```rust
// Simple derive using framework
#[proc_macro_derive(Display, attributes(display))]
pub fn derive_display(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    display_impl::expand(input).into()
}

mod display_impl {
    use macrokid_core::{ir::TypeSpec, builders::ImplBuilder, attrs::attr_string_value};
    
    pub fn expand(input: DeriveInput) -> TokenStream2 {
        // 1. Parse into normalized IR
        let spec = TypeSpec::from_derive_input(input)?;
        
        // 2. Generate logic using framework utilities
        let body = match &spec.kind {
            TypeKind::Enum(en) => {
                let arms = en.variants.iter().map(|v| {
                    let name = attr_string_value(&v.attrs, "display")
                        .unwrap_or_else(|| v.ident.to_string());
                    quote! { Self::#v_ident { .. } => f.write_str(#name) }
                });
                quote! { match self { #(#arms),* } }
            }
            TypeKind::Struct(_) => {
                let name = attr_string_value(&spec.attrs, "display")
                    .unwrap_or_else(|| spec.ident.to_string());
                quote! { f.write_str(#name) }
            }
        };
        
        // 3. Generate impl using builder
        ImplBuilder::new(spec.ident, spec.generics)
            .implement_trait(quote! { ::core::fmt::Display })
            .add_method(quote! {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #body
                }
            })
            .build()
    }
}
```

## 🔄 Data Flow

### Derive Macro Processing Flow

1. **Input Parsing**: `syn::DeriveInput` → `TypeSpec::from_derive_input()`
2. **IR Generation**: Complex AST → Normalized `TypeSpec`
3. **Logic Implementation**: Pattern match on `TypeKind` and `FieldKind`
4. **Code Generation**: Use `ImplBuilder` to create impl blocks
5. **Output**: `TokenStream2` → `TokenStream`

### Attribute Parsing Flow

1. **Raw Attributes**: `Vec<syn::Attribute>`
2. **Framework Parsing**: `attr_string_value()` or `parse_nested_attrs()`
3. **Structured Data**: `String`, `Vec<(String, String)>`, etc.
4. **Logic Integration**: Use parsed data in code generation

### Error Handling Flow

1. **syn Errors**: Proper spans preserved through framework
2. **Framework Errors**: Contextual messages with `meta.error()`  
3. **Compilation Errors**: `syn::Error::to_compile_error()`
4. **User Experience**: Clear error messages pointing to source

## 🧠 Key Technical Decisions

### 1. **Syn 2.x Migration Strategy**

**Problem**: syn 1.x used `NestedMeta`, removed in syn 2.x
**Solution**: Embrace `ParseNestedMeta` for better error handling

```rust
// syn 1.x approach (deprecated)
attr.parse_meta()? // Returns NestedMeta

// syn 2.x approach (current)  
attr.parse_nested_meta(|meta| {
    if let Some(ident) = meta.path.get_ident() {
        let value: syn::LitStr = meta.value()?.parse()?;
        // Handle parsed data
    }
    Ok(())
})?
```

**Benefits**: Better error spans, more flexible parsing, future-proof

### 2. **IR Design Philosophy**

**Problem**: syn AST is complex and varies between similar constructs
**Solution**: Normalize to essential information in `TypeSpec`

```rust
// Before: Different handling for different field types
match &data.fields {
    syn::Fields::Named(named) => { /* handle named */ },
    syn::Fields::Unnamed(unnamed) => { /* handle tuple */ },  
    syn::Fields::Unit => { /* handle unit */ },
}

// After: Unified handling through FieldKind
match &struct_spec.fields {
    FieldKind::Named(fields) => { /* unified handling */ },
    FieldKind::Unnamed(fields) => { /* same interface */ },
    FieldKind::Unit => { /* consistent pattern */ },
}
```

**Benefits**: Reduces boilerplate, consistent patterns, easier testing

### 3. **Builder Pattern Implementation**

**Problem**: `quote!` macro calls can become complex and hard to read
**Solution**: Fluent builders for common patterns

```rust
// Without builders (complex)
quote! {
    impl #impl_generics #trait_name for #type_name #ty_generics #where_clause {
        #method1
        #method2
        // ...
    }
}

// With builders (clear intent)
ImplBuilder::new(type_name, generics)
    .implement_trait(trait_name)  
    .add_method(method1)
    .add_method(method2)
    .build()
```

**Benefits**: Self-documenting, composable, prevents syntax errors

### 4. **Example-Driven Framework Design**

**Problem**: Framework features might not match real-world usage
**Solution**: Build examples alongside framework, iterate based on usage

**Process**:
1. Implement macro manually with syn/quote
2. Identify repetitive patterns  
3. Abstract patterns into framework utilities
4. Rewrite macro using framework
5. Validate framework meets real needs

## 🚧 Extension Points

### Adding New Macro Types

1. **Create Module**: `macrokid_core/src/new_type/`
2. **Implement Expansion**: Core logic using IR and builders
3. **Add Entry Point**: Thin wrapper in `macrokid/src/lib.rs`
4. **Create Example**: Demonstrate usage in `examples/`

### Adding New Builders

1. **Identify Pattern**: Common code generation needs
2. **Design API**: Fluent interface matching mental model
3. **Implement Builder**: In `macrokid_core/src/common/builders.rs`
4. **Document Usage**: Examples and API documentation

### Adding Attribute Support

1. **Parse Attributes**: Extend `common/attrs.rs` utilities
2. **Handle Edge Cases**: Empty values, invalid syntax, etc.
3. **Generate Code**: Use parsed attributes in logic
4. **Test Thoroughly**: Attribute parsing is error-prone

## 📊 Performance Considerations

### Compile-Time Performance

- **IR Overhead**: Minimal - single pass conversion from syn AST
- **Builder Overhead**: Zero - compile-time code generation
- **Framework Abstraction**: Thin layer, minimal runtime impact

### Memory Usage

- **IR Storage**: Temporary during macro expansion only
- **Generated Code**: Equivalent to hand-written implementations
- **Compilation**: Standard proc-macro memory patterns

### Optimization Opportunities

1. **Caching**: Reuse TypeSpec for multiple macro applications
2. **Lazy Generation**: Generate code sections on-demand
3. **Parallel Processing**: Handle multiple items concurrently

## 🔍 Debugging and Development

### Framework Development Tips

1. **Use `cargo expand`**: See generated code output
2. **Test IR Conversion**: Verify TypeSpec matches expectations  
3. **Validate Builders**: Check generated syntax is correct
4. **Error Message Quality**: Ensure spans point to right locations

### Common Pitfalls

1. **Attribute Parsing**: Handle edge cases (empty, malformed)
2. **Generic Handling**: Properly split and recombine generic parameters
3. **Span Preservation**: Maintain error location context
4. **TokenStream Conversion**: Between proc-macro and proc-macro2

### Testing Strategies

1. **Unit Tests**: Individual IR components and utilities
2. **Integration Tests**: Full macro expansion pipelines
3. **UI Tests**: Error message verification with `trybuild`
4. **Example Tests**: Ensure examples compile and run

---

This architecture provides a solid foundation for procedural macro development while remaining flexible for future enhancements and use cases.
