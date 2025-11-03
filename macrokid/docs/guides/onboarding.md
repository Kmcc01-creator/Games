# Developer Onboarding Guide

Welcome to Macrokid! This guide will help you get up to speed with the framework, understand its capabilities, and start building your own procedural macros.

## 🚀 Prerequisites

Before diving in, make sure you have:

- **Rust 1.70+** with 2021 edition
- **Basic proc-macro knowledge** - understanding of `#[derive]`, `#[attribute]`, and `function!()` macros
- **Familiarity with syn/quote** (helpful but not required - the framework abstracts this!)

## 📚 Learning Path

### Phase 1: Understanding the Framework (30 minutes)

1. **Explore the Examples**
   ```bash
   git clone <macrokid-repo>
   cd macrokid
   cargo run --bin example
   ```
   
   This shows all three macro types in action:
   - Function-like: `make_enum!(Color: Red, Green, Blue)`
   - Attribute: `#[trace]` for function timing
   - Derive: `#[derive(Display)]` with custom attributes

2. **Study the Output**
   ```
   Color from str: Green
   Mode: Fast  
   Mode (custom): SLOW
   Struct display: Point2D
   Config (verbose debug): CustomConfig { name: "MyApp", port: 8080 }
   work returned 999
   [macrokid::trace] work took 5.079µs
   ```

3. **Review the Architecture**
   - Read `ARCHITECTURE.md` for technical details
   - Understand the IR (Intermediate Representation) concept
   - Learn about the builder patterns

### Phase 2: Hands-On Development (60 minutes)

#### Exercise 1: Simple Derive Macro

Create a custom derive that implements a `Name` trait:

1. **Set up your crate**:
   ```toml
   [package]
   name = "my_macros"
   version = "0.1.0"
   edition = "2021"
   
   [lib]
   proc-macro = true
   
   [dependencies]
   proc-macro2 = "1"
   quote = "1"
   syn = { version = "2", features = ["full"] }
   macrokid_core = { path = "../macrokid_core" }
   ```

2. **Implement the derive**:
   ```rust
   use proc_macro::TokenStream;
   use syn::{parse_macro_input, DeriveInput};
   use macrokid_core::{ir::TypeSpec, builders::ImplBuilder};
   use quote::quote;
   
   #[proc_macro_derive(Name)]
   pub fn derive_name(input: TokenStream) -> TokenStream {
       let input: DeriveInput = parse_macro_input!(input);
       
       // Step 1: Parse into framework IR
       let spec = match TypeSpec::from_derive_input(input) {
           Ok(spec) => spec,
           Err(e) => return e.to_compile_error().into(),
       };
       
       // Step 2: Generate implementation using builder
       let name_str = spec.ident.to_string();
       let impl_block = ImplBuilder::new(spec.ident, spec.generics)
           .implement_trait(quote! { Name })
           .add_method(quote! {
               fn name(&self) -> &'static str {
                   #name_str
               }
           })
           .build();
       
       impl_block.into()
   }
   ```

3. **Test it**:
   ```rust
   use my_macros::Name;
   
   #[derive(Name)]
   struct User {
       id: u32,
   }
   
   fn main() {
       let user = User { id: 1 };
       println!("Type name: {}", user.name()); // "User"
   }
   ```

#### Exercise 2: Attribute Parsing

Enhance your derive to support custom names:

1. **Update the derive declaration**:
   ```rust
   #[proc_macro_derive(Name, attributes(name))]
   pub fn derive_name(input: TokenStream) -> TokenStream {
       // Implementation...
   }
   ```

2. **Add attribute parsing**:
   ```rust
   use macrokid_core::attrs::attr_string_value;
   
   // Inside your derive implementation:
   let custom_name = attr_string_value(&spec.attrs, "name")
       .unwrap_or_else(|| spec.ident.to_string());
   
   let impl_block = ImplBuilder::new(spec.ident, spec.generics)
       .implement_trait(quote! { Name })
       .add_method(quote! {
           fn name(&self) -> &'static str {
               #custom_name
           }
       })
       .build();
   ```

3. **Test custom attributes**:
   ```rust
   #[derive(Name)]
   #[name("CustomUser")]
   struct User {
       id: u32,
   }
   
   fn main() {
       let user = User { id: 1 };
       println!("Type name: {}", user.name()); // "CustomUser"
   }
   ```

### Phase 3: Advanced Features (90 minutes)

#### Exercise 3: Working with Enums

Create a derive that works differently for structs vs enums:

```rust
use macrokid_core::ir::{TypeKind, FieldKind};

// In your derive implementation:
let name_value = match &spec.kind {
    TypeKind::Struct(_) => {
        // For structs: use type name
        attr_string_value(&spec.attrs, "name")
            .unwrap_or_else(|| spec.ident.to_string())
    },
    TypeKind::Enum(enum_spec) => {
        // For enums: list all variants
        let variant_names: Vec<String> = enum_spec.variants
            .iter()
            .map(|v| v.ident.to_string())
            .collect();
        format!("{}({})", spec.ident, variant_names.join("|"))
    }
};
```

#### Exercise 4: Field Introspection

Create a derive that counts fields:

```rust
use macrokid_core::ir::{TypeKind, FieldKind};

let field_count = match &spec.kind {
    TypeKind::Struct(struct_spec) => {
        match &struct_spec.fields {
            FieldKind::Named(fields) => fields.len(),
            FieldKind::Unnamed(fields) => fields.len(), 
            FieldKind::Unit => 0,
        }
    },
    TypeKind::Enum(enum_spec) => {
        enum_spec.variants.len()
    }
};

let impl_block = ImplBuilder::new(spec.ident, spec.generics)
    .implement_trait(quote! { FieldCount })
    .add_method(quote! {
        fn field_count() -> usize {
            #field_count
        }
    })
    .build();
```

#### Exercise 5: Complex Attribute Parsing

Handle nested attributes like `#[config(key = "value", enabled)]`:

```rust
use macrokid_core::attrs::{parse_nested_attrs, has_attr};

let nested_attrs = parse_nested_attrs(&spec.attrs, "config").unwrap_or_default();
let is_enabled = has_attr(&spec.attrs, "enabled");

for (key, value) in &nested_attrs {
    match key.as_str() {
        "key" => {
            // Use the key value
        },
        "mode" => {
            // Use the mode value
        },
        _ => {
            // Unknown key, perhaps emit a warning
        }
    }
}
```

## 🛠️ Common Patterns and Best Practices

### Pattern 1: Error Handling

Always handle parsing errors gracefully:

```rust
#[proc_macro_derive(MyTrait)]
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    match derive_my_trait_impl(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_my_trait_impl(input: TokenStream) -> syn::Result<TokenStream> {
    // Note: At proc-macro entry points prefer `parse_macro_input!` for best diagnostics.
    // Inside helper functions it's fine to use `syn::parse`/`parse::<T>()` as shown here.
    let input: DeriveInput = syn::parse(input)?;
    let spec = TypeSpec::from_derive_input(input)?;
    
    // Your logic here...
    
    Ok(generated_tokens.into())
}

> Note: Use `parse_macro_input!` in proc-macro entry functions and `syn::parse` in
> internal helpers. Entry points benefit from span-aware diagnostics produced by
> `parse_macro_input!`, while helpers often operate on already-typed inputs.
```

### Pattern 2: Generics Handling

The framework handles generics automatically:

```rust
// This works with generic types automatically:
let impl_block = ImplBuilder::new(spec.ident, spec.generics)
    .implement_trait(quote! { MyTrait })
    .add_method(quote! { /* method implementation */ })
    .build();

// Generated code properly handles:
// struct MyType<T, U: Clone> { ... }
// impl<T, U: Clone> MyTrait for MyType<T, U> { ... }
```

### Pattern 3: Attribute Validation

Validate attributes early to provide good error messages:

```rust
use macrokid_core::attrs::{attr_string_value, has_attr};

// Validate required attributes
let name = attr_string_value(&spec.attrs, "name")
    .ok_or_else(|| {
        syn::Error::new_spanned(&spec.ident, "missing #[name(\"..\")] attribute")
    })?;

// Validate mutually exclusive attributes
if has_attr(&spec.attrs, "auto") && has_attr(&spec.attrs, "manual") {
    return Err(syn::Error::new_spanned(
        &spec.ident, 
        "cannot specify both #[auto] and #[manual]"
    ));
}
```

### Pattern 4: Multiple Method Generation

Generate multiple methods efficiently:

```rust
let mut methods = Vec::new();

// Add getter methods for all fields
if let TypeKind::Struct(struct_spec) = &spec.kind {
    if let FieldKind::Named(fields) = &struct_spec.fields {
        for field in fields {
            let field_ident = field.ident.as_ref().unwrap();
            let getter_name = format!("get_{}", field_ident);
            let getter_ident = syn::Ident::new(&getter_name, field_ident.span());
            
            methods.push(quote! {
                pub fn #getter_ident(&self) -> &_ {
                    &self.#field_ident
                }
            });
        }
    }
}

let mut builder = ImplBuilder::new(spec.ident, spec.generics);
for method in methods {
    builder = builder.add_method(method);
}
let impl_block = builder.build();
```

## 🧪 Testing Your Macros

### Unit Testing

Test individual components:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse_quote;
    
    #[test]
    fn test_simple_struct() {
        let input: DeriveInput = parse_quote! {
            struct User {
                name: String,
            }
        };
        
        let spec = TypeSpec::from_derive_input(input).unwrap();
        assert_eq!(spec.ident, "User");
        
        if let TypeKind::Struct(struct_spec) = spec.kind {
            if let FieldKind::Named(fields) = struct_spec.fields {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].ident.as_ref().unwrap(), "name");
            } else {
                panic!("Expected named fields");
            }
        } else {
            panic!("Expected struct");
        }
    }
}
```

### Integration Testing

Test full macro expansion:

```rust
#[test]  
fn test_derive_expansion() {
    let input = quote! {
        #[derive(Name)]
        #[name("TestStruct")]
        struct TestStruct {
            field: i32,
        }
    };
    
    let output = derive_name(input.into());
    
    // Test that output compiles and works as expected
    let expected = quote! {
        impl Name for TestStruct {
            fn name(&self) -> &'static str {
                "TestStruct"
            }
        }
    };
    
    // Compare token streams (you may need additional utilities for this)
}
```

### UI Testing with `trybuild`

Test error messages:

```toml
[dev-dependencies]
trybuild = "1.0"
```

```rust
#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
```

Create test files that should fail:

```rust
// tests/ui/missing_attribute.rs
use my_macros::Name;

#[derive(Name)] // Missing required #[name("...")] attribute
struct BadStruct;

fn main() {}
```

## 🔍 Debugging Tips

### Use `cargo expand`

Install and use cargo-expand to see generated code:

```bash
cargo install cargo-expand
cargo expand --test my_test
```

### Add Debug Prints

Temporarily add debug output to your macros:

```rust
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    eprintln!("Input: {}", input);
    
    // Your logic...
    
    eprintln!("Output: {}", output);
    output
}
```

### Test with Simple Cases First

Start with the simplest possible inputs:

```rust
// Start with this:
struct Simple;

// Then move to:
struct WithFields { field: i32 }

// Then:
struct Generic<T> { field: T }

// Finally:
struct Complex<T: Clone, U> where U: Default { 
    field1: T, 
    field2: U 
}
```

## 🎯 Next Steps

### Explore Advanced Framework Features

1. **MatchArmBuilder**: Generate complex match expressions
2. **Complex Attribute Parsing**: Handle deeply nested attributes
3. **Error Context**: Provide detailed error messages with spans
4. **Performance**: Understand compilation time implications
5. **Field Types in IR**: `FieldSpec` includes `ty: syn::Type` for type-driven codegen (e.g., typed getters, validation).

### Study the Examples

1. **`examples/custom_derive`**: Advanced derive patterns
2. **`macrokid_core/src/attr/trace.rs`**: Attribute macro implementation  
3. **`macrokid_core/src/function/make_enum.rs`**: Function-like macro patterns

### Build Real Macros

Start with simple utilities you actually need:
- **Getters/Setters**: Auto-generate field accessors
- **Builder Pattern**: Generate builder structs
- **Serialization**: Custom serialize/deserialize logic
- **Validation**: Auto-generate validation methods

### Contribute Back

1. **Add Utilities**: New builders or attribute parsers
2. **Create Examples**: Show new patterns and use cases  
3. **Improve Documentation**: Help other developers learn
4. **Report Issues**: Help improve the framework

## 🤝 Getting Help

- **Issues**: Open GitHub issues for bugs or feature requests
- **Discussions**: Ask questions in GitHub discussions
- **Examples**: Check existing examples for patterns
- **Source**: Read the framework source - it's designed to be readable!

Welcome to the Macrokid community! 🚀
