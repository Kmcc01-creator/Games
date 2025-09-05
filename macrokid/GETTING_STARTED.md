# Getting Started with Macrokid

A 15-minute quickstart guide to building your first procedural macro with the Macrokid framework.

## 🚀 What You'll Build

By the end of this tutorial, you'll have:
- A custom `#[derive(Greet)]` macro that generates greeting methods
- Support for customization via `#[greet("Custom message")]` attributes
- A working example showing both default and custom behavior

## 📋 Prerequisites

- Rust 1.70+ installed
- Basic familiarity with Rust structs and traits
- 15 minutes of time

## 🛠️ Step 1: Project Setup (2 minutes)

Create a new library crate:

```bash
cargo new my_greet_macro --lib
cd my_greet_macro
```

Update `Cargo.toml`:

```toml
[package]
name = "my_greet_macro"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
proc-macro2 = "1"
quote = "1" 
syn = { version = "2", features = ["full"] }
macrokid_core = { git = "https://github.com/your-org/macrokid.git" }
# Or use: macrokid_core = { path = "../macrokid/macrokid_core" }
```

## 🎯 Step 2: Basic Implementation (5 minutes)

Replace `src/lib.rs` with:

```rust
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use macrokid_core::{
    ir::{TypeSpec, TypeKind},
    builders::ImplBuilder,
    attrs::attr_string_value,
};
use quote::quote;

/// Derive macro that generates a greet() method
#[proc_macro_derive(Greet, attributes(greet))]
pub fn derive_greet(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    
    match greet_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

fn greet_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    // Step 1: Parse input into framework's IR
    let spec = TypeSpec::from_derive_input(input)?;
    
    // Step 2: Extract custom greeting or use default
    let greeting = attr_string_value(&spec.attrs, "greet")
        .unwrap_or_else(|| format!("Hello from {}!", spec.ident));
    
    // Step 3: Generate implementation using builder
    let impl_block = ImplBuilder::new(spec.ident, spec.generics)
        .implement_trait(quote! { Greet })
        .add_method(quote! {
            fn greet(&self) -> String {
                #greeting.to_string()
            }
        })
        .build();
    
    Ok(impl_block.into())
}

/// The trait our derive macro implements
pub trait Greet {
    fn greet(&self) -> String;
}
```

## 🧪 Step 3: Test It Out (3 minutes)

Create `examples/test_greet.rs`:

```rust
use my_greet_macro::{Greet, derive_greet};

// Basic usage - will use default greeting
#[derive(Greet)]
struct User {
    name: String,
}

// Custom greeting via attribute
#[derive(Greet)]
#[greet("Welcome, honored guest!")]
struct VipUser {
    name: String,
}

fn main() {
    let user = User { 
        name: "Alice".to_string() 
    };
    println!("{}", user.greet()); // "Hello from User!"
    
    let vip = VipUser { 
        name: "Bob".to_string() 
    };
    println!("{}", vip.greet()); // "Welcome, honored guest!"
}
```

Run it:

```bash
cargo run --example test_greet
```

Expected output:
```
Hello from User!
Welcome, honored guest!
```

## 🎨 Step 4: Add Advanced Features (5 minutes)

Let's enhance our macro to support field-based greetings. Update `greet_impl`:

```rust
fn greet_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    
    // Check for custom attribute first
    if let Some(custom_greeting) = attr_string_value(&spec.attrs, "greet") {
        let impl_block = ImplBuilder::new(spec.ident, spec.generics)
            .implement_trait(quote! { Greet })
            .add_method(quote! {
                fn greet(&self) -> String {
                    #custom_greeting.to_string()
                }
            })
            .build();
        return Ok(impl_block.into());
    }
    
    // Generate field-based greeting for structs
    let greeting_logic = match &spec.kind {
        TypeKind::Struct(struct_spec) => {
            match &struct_spec.fields {
                macrokid_core::ir::FieldKind::Named(fields) => {
                    // Look for a "name" field to personalize greeting
                    let has_name_field = fields.iter().any(|f| {
                        f.ident.as_ref().map(|i| i == "name").unwrap_or(false)
                    });
                    
                    if has_name_field {
                        quote! {
                            format!("Hello, {}!", self.name)
                        }
                    } else {
                        let type_name = spec.ident.to_string();
                        quote! {
                            format!("Hello from {}!", #type_name)
                        }
                    }
                },
                _ => {
                    let type_name = spec.ident.to_string();
                    quote! {
                        format!("Hello from {}!", #type_name)
                    }
                }
            }
        },
        TypeKind::Enum(_) => {
            let type_name = spec.ident.to_string();
            quote! {
                format!("Hello from {} enum!", #type_name)
            }
        }
    };
    
    let impl_block = ImplBuilder::new(spec.ident, spec.generics)
        .implement_trait(quote! { Greet })
        .add_method(quote! {
            fn greet(&self) -> String {
                #greeting_logic
            }
        })
        .build();
    
    Ok(impl_block.into())
}
```

Update your example to test the new behavior:

```rust
// This will now use the name field!
#[derive(Greet)]
struct PersonalUser {
    name: String,
    age: u32,
}

// Test enum support
#[derive(Greet)]
enum Status {
    Active,
    Inactive,
}

fn main() {
    // Previous examples...
    
    let personal = PersonalUser { 
        name: "Charlie".to_string(),
        age: 25,
    };
    println!("{}", personal.greet()); // "Hello, Charlie!"
    
    let status = Status::Active;
    println!("{}", status.greet()); // "Hello from Status enum!"
}
```

## 🎉 What You've Accomplished

In just 15 minutes, you've:

✅ **Built a real proc-macro** using the Macrokid framework  
✅ **Used modern syn 2.x APIs** without dealing with parsing complexity  
✅ **Implemented attribute support** for customization  
✅ **Added field introspection** for intelligent behavior  
✅ **Supported multiple type kinds** (structs and enums)

## 🚀 Next Steps

### Immediate Improvements

1. **Error handling**: Add better validation for invalid attributes
2. **More attributes**: Support `#[greet(prefix = "Hi", suffix = "!")]`  
3. **Field attributes**: Allow `#[greet(skip)]` on individual fields

### Learn More

1. **[Onboarding Guide](ONBOARDING.md)**: Complete learning path with advanced examples
2. **[API Reference](API_REFERENCE.md)**: Full documentation of all framework features
3. **[Architecture](ARCHITECTURE.md)**: Deep dive into framework design
4. **Example Projects**: Study `examples/custom_derive` for advanced patterns

### Contribute

- Add new builder patterns to the framework
- Create example macros for common use cases
- Improve documentation and tutorials
- Report issues and suggest improvements

## 💡 Key Framework Benefits You Used

| Without Macrokid | With Macrokid |
|------------------|---------------|
| Manual syn AST parsing | `TypeSpec::from_derive_input()` |
| Complex quote! macro calls | `ImplBuilder` fluent API |
| Custom attribute parsing | `attr_string_value()` |
| Error-prone span handling | Automatic error context |
| Boilerplate for each macro | Reusable framework components |

## 🤝 Community

- **GitHub Issues**: Report bugs and request features
- **Discussions**: Ask questions and share your macros
- **Examples**: Contribute real-world macro examples
- **Documentation**: Help improve guides and tutorials

Congratulations! You've successfully built your first Macrokid-powered procedural macro! 🎉

---

Ready to build more complex macros? Check out the [Onboarding Guide](ONBOARDING.md) for advanced techniques and patterns.