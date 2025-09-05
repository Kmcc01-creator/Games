# Macrokid Core API Reference

Complete API documentation for `macrokid_core` - the framework foundation for building procedural macros.

## 🔁 Version Compatibility

- Crate versions: `syn = "2"`, `quote = "1"`, `proc-macro2 = "1"`.
- MSRV: Rust 1.70+ (edition 2021).
- Framework APIs are written against syn 2.x; syn 1.x-only patterns are intentionally avoided.

### Common syn 1 → 2 Migration Errors

- Using `attr.parse_meta()` or matching on `NestedMeta` (removed). Use `attr.parse_nested_meta(|meta| { ... })`.
- Parsing bare string lists in attributes: use `meta.input.parse::<syn::LitStr>()?` for `#[attr("a", "b")]`.
- Parsing name-value pairs: use `meta.value()?.parse::<syn::LitStr>()?` for `#[attr(key = "val")]`.
- Accessing attribute internals directly: prefer `attr.path().is_ident("...")` and helper methods over struct fields.

These details are encapsulated by `common::attrs` helpers so you can stay high-level.

## 📚 Module Overview

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `ir` | Type introspection and normalization | `TypeSpec`, `TypeKind`, `FieldKind` |
| `common::attrs` | Attribute parsing utilities | Functions for parsing various attribute patterns |
| `common::builders` | Code generation helpers | `ImplBuilder`, `MatchArmBuilder` |
| `common::patterns` | Semantic match helpers | `match_variants`, `match_fields` |
| `common::diag` | Diagnostics utilities | `err_on`, `suggest_with_note` |
| `common::type_utils` | Type inspection helpers | `is_option`, `unwrap_vec`, `unwrap_result` |
| `common::repr` | Repr parsing | `ReprInfo`, `parse_repr` |
| `common::pattern_dsl` (feature) | Typed pattern builder | `PatternSpec`, `StructFields` |
| `common::type_utils` | Type inspection helpers | `is_option`, `unwrap_vec`, `unwrap_result` |
| `common::repr` | Repr parsing | `ReprInfo`, `parse_repr` |
| `attr` | Attribute macro helpers | Ready-to-use attribute implementations |
| `function` | Function-like macro helpers | Parsers and generators for function macros |

## 🏗️ Core IR (Intermediate Representation)

### `TypeSpec`

The primary type for representing normalized Rust types.

```rust
pub struct TypeSpec {
    pub ident: Ident,           // Type name (e.g., "MyStruct")
    pub generics: Generics,     // Generic parameters and constraints
    pub attrs: Vec<Attribute>,  // Type-level attributes
    pub vis: Visibility,        // Visibility of the type (IR-B)
    pub span: Span,             // Container span (IR-B)
    pub kind: TypeKind,         // Struct or Enum with details
}
```

#### Methods

```rust
impl TypeSpec {
    /// Parse a DeriveInput into normalized TypeSpec
    pub fn from_derive_input(input: DeriveInput) -> syn::Result<Self>
}
```

**Example Usage:**
```rust
use macrokid_core::ir::TypeSpec;

let spec = TypeSpec::from_derive_input(input)?;
println!("Type name: {}", spec.ident);
match spec.kind {
    TypeKind::Struct(_) => println!("It's a struct!"),
    TypeKind::Enum(_) => println!("It's an enum!"),
}
```

### Supported Input Kinds and Considerations

- Supported: structs and enums. Unions are rejected with a clear error.
- Fields: named, unnamed (tuple), and unit are represented via `FieldKind`.
- Generics: preserved, including lifetimes and where-clauses; builders handle them via `split_for_impl()`.
- Attributes: available at type, variant, and field level for parsing via `common::attrs`.
- Field types: not included in `FieldSpec` yet; extend IR if needed for type-driven macros.

### `TypeKind`

Discriminates between struct and enum types.

```rust
pub enum TypeKind {
    Struct(StructSpec),
    Enum(EnumSpec),
}
```

### `StructSpec`

Information about struct types.

```rust
pub struct StructSpec {
    pub fields: FieldKind,  // Field information
}
```

### `EnumSpec` 

Information about enum types.

```rust
pub struct EnumSpec {
    pub variants: Vec<VariantSpec>,  // All enum variants
}
```

### `VariantSpec`

Information about individual enum variants.

```rust
pub struct VariantSpec {
    pub ident: Ident,           // Variant name
    pub attrs: Vec<Attribute>,  // Variant-level attributes
    pub span: Span,             // Variant span (IR-B)
    pub discriminant: Option<Expr>, // Optional explicit value (IR-B)
    pub fields: FieldKind,      // Variant field information
}
```

### `FieldKind`

Unified representation of field types.

```rust
pub enum FieldKind {
    Named(Vec<FieldSpec>),      // struct { name: Type }
    Unnamed(Vec<FieldSpec>),    // struct(Type1, Type2)  
    Unit,                       // struct;
}
```

#### Methods

```rust
impl FieldKind {
    /// Create FieldKind from syn::Fields
    pub fn from_fields(fields: Fields) -> syn::Result<Self>
}
```

### `FieldSpec`

Information about individual fields.

```rust
pub struct FieldSpec {
    pub ident: Option<Ident>,   // Field name (None for tuple fields)
    pub index: usize,           // Position in field list
    pub attrs: Vec<Attribute>,  // Field-level attributes  
    pub vis: Visibility,        // Field visibility (IR-B)
    pub ty: Type,               // Field type (new in this version)
    pub span: Span,             // Source location
}
```

**Example: Field Processing**
```rust
use macrokid_core::ir::{FieldKind, TypeKind};

match &spec.kind {
    TypeKind::Struct(struct_spec) => {
        match &struct_spec.fields {
            FieldKind::Named(fields) => {
                for field in fields {
                    let name = field.ident.as_ref().unwrap();
                    println!("Field: {}", name);
                }
            },
            FieldKind::Unnamed(fields) => {
                println!("Tuple struct with {} fields", fields.len());
            },
            FieldKind::Unit => {
                println!("Unit struct");
            }
        }
    }
}
```

## 🔧 Common Utilities

### `common::attrs` - Attribute Parsing

#### `attr_string_value`

Extract a string value from attributes like `#[attr = "value"]` or `#[attr("value")]`.

```rust
pub fn attr_string_value(attrs: &[Attribute], attr_name: &str) -> Option<String>
```

**Supported Patterns:**
- `#[display = "CustomName"]` 
- `#[display("CustomName")]`

**Example:**
```rust
use macrokid_core::attrs::attr_string_value;

let custom_name = attr_string_value(&spec.attrs, "display")
    .unwrap_or_else(|| spec.ident.to_string());
```

#### `attr_string_list`

Extract multiple string values from attributes like `#[attr("val1", "val2")]`.

```rust
pub fn attr_string_list(attrs: &[Attribute], attr_name: &str) -> Option<Vec<String>>
```

**Example:**
```rust
let features = attr_string_list(&spec.attrs, "features")
    .unwrap_or_default();
for feature in features {
    println!("Feature: {}", feature);
}
```

#### `has_attr`

Check if an attribute exists.

```rust
pub fn has_attr(attrs: &[Attribute], attr_name: &str) -> bool
```

**Example:**
```rust
if has_attr(&field.attrs, "skip") {
    // Skip this field
    continue;
}
```

#### `parse_nested_attrs`

Parse complex nested attributes like `#[config(key = "value", mode = "fast")]`.

```rust
pub fn parse_nested_attrs(attrs: &[Attribute], attr_name: &str) -> syn::Result<Vec<(String, String)>>
```

**Example:**
```rust
let config = parse_nested_attrs(&spec.attrs, "config")?;
for (key, value) in config {
    match key.as_str() {
        "mode" => println!("Mode: {}", value),
        "level" => println!("Level: {}", value),
        _ => eprintln!("Unknown config key: {}", key),
    }
}
```

#### `get_nested_attr_value`

Get a specific key from nested attributes.

```rust
pub fn get_nested_attr_value(attrs: &[Attribute], attr_name: &str, key: &str) -> Option<String>
```

**Example:**
```rust
let mode = get_nested_attr_value(&spec.attrs, "config", "mode")
    .unwrap_or_else(|| "default".to_string());
```

### `common::builders` - Code Generation

#### `ImplBuilder`

Fluent API for generating implementation blocks.

```rust
pub struct ImplBuilder {
    // Internal fields...
}
```

##### Methods

```rust
impl ImplBuilder {
    /// Create a new implementation builder
    pub fn new(target_type: Ident, generics: Generics) -> Self
    
    /// Add a trait implementation
    pub fn implement_trait(self, trait_name: TokenStream2) -> Self
    
    /// Add a method to the implementation
    pub fn add_method(self, method: TokenStream2) -> Self

    /// Add an associated type: `type Name = Ty;`
    pub fn add_assoc_type(self, name: Ident, ty: TokenStream2) -> Self

    /// Add an associated const: `const NAME: Ty = VALUE;`
    pub fn add_assoc_const(self, name: Ident, ty: TokenStream2, value: TokenStream2) -> Self

    /// Attach doc comments to the impl block
    pub fn with_docs(self, docs: &str) -> Self

    /// Attach arbitrary attributes to the impl block (e.g., cfg, allow)
    pub fn with_attrs(self, attrs: TokenStream2) -> Self
    
    /// Build the final implementation block
    pub fn build(self) -> TokenStream2
}
```

**Example: Trait Implementation**
```rust
use macrokid_core::builders::ImplBuilder;
use quote::quote;

let impl_block = ImplBuilder::new(spec.ident.clone(), spec.generics)
    .implement_trait(quote! { std::fmt::Display })
    .add_method(quote! {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", "MyType")
        }
    })
    .build();
```

**Example: Inherent Implementation**
```rust
let impl_block = ImplBuilder::new(spec.ident.clone(), spec.generics)
    .add_method(quote! {
        pub fn new() -> Self {
            Self { /* initialization */ }
        }
    })
    .add_method(quote! {
        pub fn do_something(&self) {
            // Method implementation
        }
    })
    .build();
```

#### `MatchArmBuilder`

Helper for generating match expressions.

```rust
pub struct MatchArmBuilder {
    // Internal fields...
}
```

##### Methods

```rust
impl MatchArmBuilder {
    /// Create a new match arm builder
    pub fn new() -> Self
    
    /// Add a match arm
    pub fn add_arm(self, pattern: TokenStream2, body: TokenStream2) -> Self
    
    /// Add a wildcard `_` arm
    pub fn add_wildcard(self, body: TokenStream2) -> Self
    
    /// Add an arm with a guard: `pattern if guard`
    pub fn add_guarded_arm(self, pattern: TokenStream2, guard: TokenStream2, body: TokenStream2) -> Self
    
    /// Add an arm combining multiple patterns with `|`
    pub fn add_multi_pattern<I>(self, patterns: I, body: TokenStream2) -> Self
    where I: IntoIterator<Item = TokenStream2>
    
    /// Build a complete match expression
    pub fn build_match(self, scrutinee: TokenStream2) -> TokenStream2
    
    /// Build just the arms (for existing match expressions)
    pub fn build_arms(self) -> Vec<TokenStream2>
}
```

**Example: Enum Display**
```rust
use macrokid_core::builders::MatchArmBuilder;

let mut builder = MatchArmBuilder::new();
for variant in &enum_spec.variants {
    let v_ident = &variant.ident;
    let v_name = v_ident.to_string();
    builder = builder.add_arm(
        quote! { Self::#v_ident { .. } },
        quote! { f.write_str(#v_name) }
    );
}

let match_expr = builder.build_match(quote! { self });
```

See MATCH_ARM_BUILDER.md for a deeper dive, advanced patterns, and roadmap.

### `common::patterns` - Semantic Helpers

Helpers that generate match arms from IR types.

```rust
use macrokid_core::patterns::{match_variants, match_fields};

// Build arms per enum variant
let arms = match_variants(&enum_spec, |v| {
    let vi = &v.ident;
    let name = vi.to_string();
    (quote! { Self::#vi { .. } }, quote! { f.write_str(#name) })
});
let display_body = arms.build_match(quote! { self });

// Build arms per struct field (named or tuple)
let getters = match_fields(&struct_spec.fields, |field| {
    if let Some(ident) = &field.ident {
        let getter_name = syn::Ident::new(&format!("get_{}", ident), ident.span());
        Some((
            quote! { Self { #ident, .. } },
            quote! { return Some(#ident) },
        ))
    } else {
        None
    }
});
let getter_body = getters.add_wildcard(quote! { None }).build_match(quote! { self });
```

Heuristic validation:
```rust
use macrokid_core::patterns::suggest_wildcard_if_non_exhaustive;

let builder = match_variants(&enum_spec, |v| { /* ... */ });
let builder = suggest_wildcard_if_non_exhaustive(builder, enum_spec.variants.len(), "non-exhaustive match");
```

### `common::diag` — Diagnostics Utilities

Span-aware error helpers for consistent macro diagnostics.

```rust
use macrokid_core::diag::{err_on, suggest_with_note, err_at_span};

// Create an error on a node with a message
let err = err_on(&spec.ident, "unsupported type");

// Add a note to guide users
let err = suggest_with_note(&spec.ident, "invalid attribute", "use #[name(\"value\")] instead");
let err2 = err_at_span(spec.span, "error at container span");
```

### `common::type_utils` — Type Inspection

Utilities for inspecting `syn::Type` paths commonly used in macros.

```rust
use macrokid_core::type_utils::{is_option, unwrap_option, is_vec, unwrap_vec, unwrap_result};

if let Some(inner) = unwrap_option(&field.ty) {
    // Handle Option<Inner>
}

if is_vec(&field.ty) {
    // Handle Vec<T>
}

if let Some((ok_ty, err_ty)) = unwrap_result(&field.ty) {
    // Handle Result<Ok, Err>
}
```

### `common::repr` — Repr Parsing

Parse `#[repr(...)]` attributes into a normalized structure.

```rust
use macrokid_core::repr::{parse_repr, ReprInfo, ReprKind, IntRepr};

if let Some(info) = parse_repr(&spec.attrs)? {
    if info.kind == Some(ReprKind::C) { /* FFI-friendly */ }
    if let Some(al) = info.align { /* alignment considerations */ }
    if let Some(ir) = info.int { /* integer repr for enums */ }
}
```

### `common::pattern_dsl` — Typed Pattern Builder (feature: `pattern_dsl`)

Build match patterns with a structured API and lower to tokens.

```rust
use macrokid_core::pattern_dsl::{PatternSpec as P, StructFields};

let pat = P::Struct { path: parse_quote!(Self::Point), fields: StructFields {
    named: vec![ (parse_quote!(x), P::Ident(parse_quote!(x))) ],
    rest: true,
}};

let guarded = pat.clone().with_guard(parse_quote!(x > 0));
let alt = pat.or(P::Wildcard);
let ts = guarded.into_tokens();
```

Enable with:
```toml
[dependencies]
macrokid_core = { version = "*", features = ["pattern_dsl"] }
```

### `common::type_utils` — Type Inspection

Utilities for inspecting `syn::Type` paths commonly used in macros.

```rust
use macrokid_core::type_utils::{is_option, unwrap_option, is_vec, unwrap_vec, unwrap_result};

if let Some(inner) = unwrap_option(&field.ty) {
    // Handle Option<Inner>
}

if is_vec(&field.ty) {
    // Handle Vec<T>
}

if let Some((ok_ty, err_ty)) = unwrap_result(&field.ty) {
    // Handle Result<Ok, Err>
}
```

## 🎯 Attribute Helpers

### `attr::trace`

Ready-to-use function tracing implementation.

#### `expand_trace`

Generate tracing wrapper for functions. Supports options via `#[trace(...)]`:

- `prefix = "..."` (string)
- `release = true|false` (bool; default true). When false, logging only in debug builds.
- `logger = "eprintln"|"log"` (string; default `eprintln`). When `log` is chosen, uses `log::trace!` if the `log` feature is enabled, otherwise falls back to `eprintln!`.

```rust
pub struct TraceConfig { pub prefix: String, pub release: bool, pub logger: TraceLogger }
pub enum TraceLogger { Eprintln, Log }
pub fn expand_trace(func: ItemFn, cfg: TraceConfig) -> TokenStream2
```

**Parameters:**
- `func`: The function to wrap
- `prefix`: Optional custom prefix for log messages

**Example:**
```rust
use macrokid_core::attr::trace;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(item as ItemFn);
    // parse options ... then call
    trace::expand_trace(func, cfg).into()
}
```

**Generated Code Pattern:**
```rust
fn original_function(args) -> ReturnType {
    let __macrokid_start = std::time::Instant::now();
    let __macrokid_ret = (|| {
        // Original function body
    })();
    eprintln!("[prefix] function_name took {:?}", __macrokid_start.elapsed());
    __macrokid_ret
}
```

Async behavior:
- In async functions, the timer starts when the async body first runs and stops on completion; elapsed time includes time across `.await` points.
- The wrapper evaluates the original body within a nested expression to ensure timing and logging occur even with early returns.

## 🔧 Function Helpers

### `function::make_enum`

#### `MakeEnumInput`

Parser for `make_enum!` syntax.

```rust
pub struct MakeEnumInput {
    pub name: Ident,                    // Enum name
    pub variants: Vec<EnumVariant>,     // Variant list
}

pub struct EnumVariant {
    pub name: Ident,                    // Variant name
    pub display_name: Option<String>,   // Custom display name
}
```

#### `expand_make_enum`

Generate enum with derived traits.

```rust
pub fn expand_make_enum(input: MakeEnumInput) -> TokenStream2
```

**Generated Traits:**
- `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`
- `std::fmt::Display`
- `std::str::FromStr`

**Example Usage:**
```rust
use macrokid_core::function::make_enum;
use syn::parse_macro_input;

#[proc_macro]
pub fn make_enum(input: TokenStream) -> TokenStream {
    let parsed: make_enum::MakeEnumInput = parse_macro_input!(input as make_enum::MakeEnumInput);
    make_enum::expand_make_enum(parsed).into()
}
```

### `function::bracket_enum`

Future support for bracket-style syntax `make_enum[Name: A, B, C]`.

## 🚨 Error Handling

### Best Practices

1. **Always handle `syn::Result`**:
   ```rust
   let spec = match TypeSpec::from_derive_input(input) {
       Ok(spec) => spec,
       Err(e) => return e.to_compile_error().into(),
   };
   ```

2. **Use proper error spans**:
   ```rust
   return Err(syn::Error::new_spanned(
       &spec.ident,
       "this type is not supported"
   ));
   ```

3. **Validate early**:
   ```rust
   if let TypeKind::Union(_) = spec.kind {
       return Err(syn::Error::new_spanned(
           &spec.ident, 
           "unions are not supported"
       ));
   }
   ```

### Common Error Patterns

```rust
// Missing required attribute
let name = attr_string_value(&spec.attrs, "name")
    .ok_or_else(|| syn::Error::new_spanned(&spec.ident, "missing #[name] attribute"))?;

// Invalid attribute combination
if has_attr(&spec.attrs, "auto") && has_attr(&spec.attrs, "manual") {
    return Err(syn::Error::new_spanned(&spec.ident, "conflicting attributes"));
}

// Unsupported type
match spec.kind {
    TypeKind::Enum(_) => { /* handle enum */ },
    TypeKind::Struct(_) => {
        return Err(syn::Error::new_spanned(&spec.ident, "structs not supported"));
    }
}
```

## 🎯 Usage Patterns

### Pattern: Simple Derive

```rust
#[proc_macro_derive(MyTrait)]
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    
    match derive_impl(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    
    let impl_block = ImplBuilder::new(spec.ident, spec.generics)
        .implement_trait(quote! { MyTrait })
        .add_method(quote! {
            fn my_method(&self) -> String {
                "Hello from macro!".to_string()
            }
        })
        .build();
    
    Ok(impl_block.into())
}
```

### Pattern: Attribute-Aware Derive

```rust
#[proc_macro_derive(Display, attributes(display))]
pub fn derive_display(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let spec = TypeSpec::from_derive_input(input).unwrap();
    
    let display_logic = match &spec.kind {
        TypeKind::Enum(enum_spec) => {
            let arms = enum_spec.variants.iter().map(|v| {
                let v_ident = &v.ident;
                let display_name = attr_string_value(&v.attrs, "display")
                    .unwrap_or_else(|| v_ident.to_string());
                quote! { Self::#v_ident { .. } => f.write_str(#display_name) }
            });
            quote! { match self { #(#arms),* } }
        },
        TypeKind::Struct(_) => {
            let display_name = attr_string_value(&spec.attrs, "display")
                .unwrap_or_else(|| spec.ident.to_string());
            quote! { f.write_str(#display_name) }
        }
    };
    
    let impl_block = ImplBuilder::new(spec.ident, spec.generics)
        .implement_trait(quote! { std::fmt::Display })
        .add_method(quote! {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #display_logic
            }
        })
        .build();
    
    impl_block.into()
}
```

### Pattern: Field Processing

```rust
fn process_struct_fields(struct_spec: &StructSpec) -> Vec<TokenStream2> {
    let mut methods = Vec::new();
    
    if let FieldKind::Named(fields) = &struct_spec.fields {
        for field in fields {
            if has_attr(&field.attrs, "skip") {
                continue;
            }
            
            let field_ident = field.ident.as_ref().unwrap();
            let getter_name = format!("get_{}", field_ident);
            let getter = syn::Ident::new(&getter_name, field_ident.span());
            
            methods.push(quote! {
                // Use recorded field type information
                pub fn #getter(&self) -> &#{ /* field.ty is a syn::Type available for advanced generation */ }
                    &self.#field_ident
                }
            });
        }
    }
    
    methods
}
```

## 📝 Examples

See the `examples/custom_derive` crate for comprehensive examples of:

- **Basic derive macros** using `TypeSpec` and `ImplBuilder`
- **Advanced attribute parsing** with nested attributes
- **Complex field processing** with conditional logic
- **Error handling** with proper spans
- **Multiple trait generation** for the same type

## 🔗 Related Documentation

- [Architecture Overview](ARCHITECTURE.md) - Technical details and design decisions
- [Onboarding Guide](ONBOARDING.md) - Step-by-step learning path  
- [Main README](README.md) - Project overview and quick start

---

This API reference covers the complete public interface of `macrokid_core`. For implementation details, see the source code which is designed to be readable and well-commented.
#### `has_flag`

Check for a marker/flag attribute (no arguments): `#[flag]`.

```rust
pub fn has_flag(attrs: &[Attribute], attr_name: &str) -> bool
```

#### `attr_bool_value`

Extract boolean values from attributes like `#[attr(true)]` or `#[attr = true]`.

```rust
pub fn attr_bool_value(attrs: &[Attribute], attr_name: &str) -> Option<bool>
```

#### `attr_int_value`

Extract integer values from attributes like `#[attr(123)]` or `#[attr = 123]`.

```rust
pub fn attr_int_value(attrs: &[Attribute], attr_name: &str) -> Option<i64>
```

#### `validate_attrs`

Validate and parse nested attributes like `#[cfgx(name = "X", enabled = true, count = 2)]` against a schema.

```rust
pub enum AttrType { Str, Bool, Int }
pub struct AttrSpec { pub key: &'static str, pub required: bool, pub ty: AttrType }
pub enum AttrValue { Str(String), Bool(bool), Int(i64) }

pub fn validate_attrs(
    attrs: &[Attribute],
    attr_name: &str,
    schema: &[AttrSpec],
) -> syn::Result<std::collections::HashMap<String, AttrValue>>
```

Errors point to precise spans; duplicate keys and unknown keys are rejected. Required keys are enforced.
**Example: Associated Items and Attributes**
```rust
let impl_block = ImplBuilder::new(spec.ident.clone(), spec.generics)
    .implement_trait(quote! { Iterator })
    .add_assoc_type(syn::Ident::new("Item", spec.ident.span()), quote! { u32 })
    .add_assoc_const(syn::Ident::new("COUNT", spec.ident.span()), quote! { usize }, quote! { 42 })
    .with_docs("Auto-generated by macrokid")
    .with_attrs(quote! { #[cfg_attr(test, allow(dead_code))] })
    .add_method(quote! { fn next(&mut self) -> Option<Self::Item> { None } })
    .build();
```
### `common::repr` — Repr Parsing

Parse `#[repr(...)]` attributes into a normalized structure.

```rust
use macrokid_core::repr::{parse_repr, ReprInfo, ReprKind, IntRepr};

if let Some(info) = parse_repr(&spec.attrs)? {
    if info.kind == Some(ReprKind::C) { /* FFI-friendly */ }
    if let Some(al) = info.align { /* alignment considerations */ }
    if let Some(ir) = info.int { /* integer repr for enums */ }
}
```
Note on struct patterns with `..`

- Background: Earlier versions emitted struct patterns using a quote repetition that mixed named fields and an optional trailing `..` using `#(, #rest )*`. `quote!` requires an actual iterator for `*`/`+` repetitions; an `Option<T>` is not an iterator, causing a compile-time error like “expected `HasIterator`, found `ThereIsNoIteratorInRepetition`”.
- Fix: Build the named fields once, detect the presence of `rest`, and then assemble one of four shapes: `{ named, .. }`, `{ named }`, `{ .. }`, or `{ }`.
- Impact: Consumers can safely enable the `pattern_dsl` feature; struct pattern expansion handles the trailing `..` correctly without relying on invalid repetitions.
