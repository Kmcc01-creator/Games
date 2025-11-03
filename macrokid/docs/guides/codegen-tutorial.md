# Macrokid Code Generation Framework Tutorial

Welcome to the macrokid code generation framework! This tutorial will show you how to build powerful, composable derive macros with minimal boilerplate.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Core Concepts](#core-concepts)
3. [Three Ways to Build Derives](#three-ways-to-build-derives)
4. [ResultCodeGen: Fallible Composition](#resultcodegen-fallible-composition)
5. [Advanced Patterns](#advanced-patterns)
6. [Real-World Examples](#real-world-examples)

---

## Quick Start

**Goal:** Build a custom derive that generates static metadata from field attributes.

**3 Lines of Code:**

```rust
use macrokid_core::derive_slice;

derive_slice! {
    pub struct MyDerive {
        descriptor_type: MyDescriptor,
        trait_path: my_crate::MyTrait,
        method: get_data,
        module: my_data,

        fn collect(spec: &TypeSpec) -> syn::Result<Vec<MyDescriptor>> {
            // Your collection logic here
            Ok(vec![/* descriptors */])
        }
    }
}

// That's it! MyDerive now implements StaticSliceDerive
```

---

## Core Concepts

### The Problem

Writing proc macros involves a lot of boilerplate:
- Parsing syn AST
- Validating input
- Generating modules with static data
- Implementing traits
- Adding inherent methods

**90% of this is repetitive** across different derives.

### The Solution

The macrokid framework provides:

1. **CodeGen Combinators** - Compose generation logic
2. **Derive Patterns** - Extract common patterns (StaticSliceDerive, StaticItemDerive)
3. **Builder API** - Configure derives dynamically
4. **Declarative DSL** - Define derives with minimal syntax

---

## Three Ways to Build Derives

### Method 1: Trait Implementation (Most Control)

Best for: Complex derives with custom generation logic.

```rust
use macrokid_core::common::derive_patterns::StaticSliceDerive;
use quote::{quote, ToTokens};

#[derive(Clone)]
struct MyDescriptor {
    name: String,
    value: u32,
}

impl ToTokens for MyDescriptor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let value = self.value;
        tokens.extend(quote! {
            MyDescriptor { name: #name, value: #value }
        });
    }
}

struct MyDerive;

impl StaticSliceDerive for MyDerive {
    type Descriptor = MyDescriptor;

    fn descriptor_type() -> TokenStream {
        quote! { my_crate::MyDescriptor }
    }

    fn collect_descriptors(spec: &TypeSpec) -> syn::Result<Vec<Self::Descriptor>> {
        // Parse field attributes
        // Validate
        // Build descriptors
        let descriptors = vec![
            MyDescriptor { name: "field1".into(), value: 1 },
            MyDescriptor { name: "field2".into(), value: 2 },
        ];
        Ok(descriptors)
    }

    fn trait_path() -> TokenStream {
        quote! { my_crate::MyTrait }
    }

    fn method_name() -> Ident {
        Ident::new("get_data", Span::call_site())
    }

    fn module_hint() -> &'static str {
        "my_data"
    }
}

// Usage in proc macro:
fn my_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    MyDerive::generate(&spec)
}
```

**Generated Code:**

```rust
mod __mk_my_data {
    pub static DATA: &[my_crate::MyDescriptor] = &[
        MyDescriptor { name: "field1", value: 1 },
        MyDescriptor { name: "field2", value: 2 },
    ];
}

impl my_crate::MyTrait for MyType {
    fn get_data() -> &'static [my_crate::MyDescriptor] {
        __mk_my_data::DATA
    }
}

impl MyType {
    pub fn describe_my_data() -> &'static [my_crate::MyDescriptor] {
        __mk_my_data::DATA
    }
}
```

---

### Method 2: Builder API (Flexible)

Best for: Dynamic configuration or when you want to avoid trait implementations.

```rust
use macrokid_core::common::derive_patterns::StaticSliceBuilder;

fn my_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;

    let builder = StaticSliceBuilder::new()
        .descriptor_type(quote! { my_crate::MyDescriptor })
        .trait_path(quote! { my_crate::MyTrait })
        .method_name("get_data")
        .module_hint("my_data")
        .collector(|spec| {
            // Collection logic
            Ok(vec![
                MyDescriptor { name: "field1".into(), value: 1 },
                MyDescriptor { name: "field2".into(), value: 2 },
            ])
        });

    builder.generate(&spec)
}
```

**Same generated code as Method 1**, but configured at runtime!

---

### Method 3: Declarative DSL (Simplest)

Best for: Quick derives, prototyping, or when you want minimal boilerplate.

```rust
use macrokid_core::derive_slice;

derive_slice! {
    /// My custom derive macro
    pub struct MyDerive {
        descriptor_type: MyDescriptor,
        trait_path: my_crate::MyTrait,
        method: get_data,
        module: my_data,
        inherent: describe_my_data,  // Optional custom inherent method name

        fn collect(spec: &TypeSpec) -> syn::Result<Vec<MyDescriptor>> {
            // Collection logic
            Ok(vec![
                MyDescriptor { name: "field1".into(), value: 1 },
                MyDescriptor { name: "field2".into(), value: 2 },
            ])
        }
    }
}

// Usage:
fn my_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    MyDerive::generate(&spec)
}
```

**One derive_slice! call** replaces 50+ lines of boilerplate!

---

## ResultCodeGen: Fallible Composition

When your generators can fail, use `ResultCodeGen` for clean error handling.

### Problem: Validation + Generation

```rust
// Without ResultCodeGen (messy):
type Derive = Chain<ValidateGen, GenerateGen>;
// Can't fail! What if validation fails?
```

### Solution: ResultCodeGen Combinators

```rust
use macrokid_core::common::gen::{ResultCodeGen, ResultChain, TryChain};

struct ValidateGen;
impl ResultCodeGen<Input> for ValidateGen {
    type Output = TokenStream;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        if !input.is_valid() {
            return Err(syn::Error::new(Span::call_site(), "invalid input"));
        }
        Ok(quote! { /* validation passed */ })
    }
}

struct GenerateGen;
impl ResultCodeGen<Input> for GenerateGen {
    type Output = TokenStream;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        Ok(quote! { /* generated code */ })
    }
}

// Compose with error propagation
type SafeDerive = ResultChain<ValidateGen, GenerateGen>;

// Use it:
let result = <SafeDerive as ResultCodeGen<Input>>::generate(&input)?;
```

### Using `try_seq!` Macro

```rust
type SafeDerive = try_seq![
    ValidateGen,
    ParseGen,
    GenerateGen,
    FinalizeGen
];

// If any step fails, error is propagated immediately
let result = <SafeDerive as ResultCodeGen<Input>>::generate(&input)?;
```

### Mixing Fallible and Infallible

```rust
use macrokid_core::common::gen::{TryChain, Lift};

struct InfallibleGen;
impl CodeGen<Input> for InfallibleGen {
    type Output = TokenStream;
    fn generate(input: &Input) -> Self::Output {
        quote! { /* always succeeds */ }
    }
}

// Chain fallible → infallible
type Mixed = TryChain<ValidateGen, InfallibleGen>;

// Or lift infallible into fallible context
type Lifted = ResultChain<Lift<InfallibleGen>, ValidateGen>;
```

---

## Advanced Patterns

### Pattern 1: Conditional Generation

```rust
use macrokid_core::common::gen::{Conditional, TryConditional};

struct IsEnum;
impl Predicate<TypeSpec> for IsEnum {
    fn test(spec: &TypeSpec) -> bool {
        matches!(spec.kind, TypeKind::Enum(_))
    }
}

// Choose generator based on input shape
type AdaptiveGen = Conditional<IsEnum, EnumGen, StructGen>;
```

### Pattern 2: Sequencing with `seq!`

```rust
type FullDerive = seq![
    ModuleGen,
    TraitImplGen,
    InherentImplGen,
    DebugImplGen
];

// Expands to: Chain<Chain<Chain<ModuleGen, TraitImplGen>, InherentImplGen>, DebugImplGen>
```

### Pattern 3: Transform-Then-Generate

```rust
use macrokid_core::common::gen::{Map, MapFn};

struct ExtractFields;
impl MapFn<TypeSpec> for ExtractFields {
    type To = Vec<FieldSpec>;
    fn map(spec: &TypeSpec) -> Vec<FieldSpec> {
        // Extract just the fields we need
        spec.fields().to_vec()
    }
}

// Transform input, then generate
type Mapped = Map<FieldGen, ExtractFields>;
```

---

## Real-World Examples

### Example 1: Resource Binding Derive (from macrokid_graphics)

**Before refactoring:** 133 lines
**After refactoring:** 60 lines

```rust
use macrokid_core::common::derive_patterns::StaticSliceDerive;

#[derive(Clone)]
struct BindingDescriptor {
    field: String,
    set: u32,
    binding: u32,
    kind: TokenStream,
    stages: Option<TokenStream>,
}

impl ToTokens for BindingDescriptor { /* ... */ }

struct ResourceBindingDerive;

impl StaticSliceDerive for ResourceBindingDerive {
    type Descriptor = BindingDescriptor;

    fn descriptor_type() -> TokenStream {
        quote! { macrokid_graphics::resources::BindingDesc }
    }

    fn collect_descriptors(spec: &TypeSpec) -> syn::Result<Vec<Self::Descriptor>> {
        // Validate struct shape
        let st = match &spec.kind {
            TypeKind::Struct(st) => st,
            _ => return Err(syn::Error::new(spec.span, "expected struct")),
        };

        // Parse exclusive attributes
        let kind_set = exclusive_schemas![
            uniform(set: int, binding: int, stages: str),
            texture(set: int, binding: int, stages: str),
            sampler(set: int, binding: int, stages: str),
            combined(set: int, binding: int, stages: str),
        ];

        // Collect descriptors from fields
        let descriptors = collect::from_named_fields(st, |f| {
            if let Some((kind_name, parsed)) = kind_set.parse(&f.attrs)? {
                let field = f.ident.as_ref().unwrap().to_string();
                let set = parsed.try_get_int("set")? as u32;
                let binding = parsed.try_get_int("binding")? as u32;
                let stages = parse_stages(parsed.get_str("stages"));

                Ok(Some(BindingDescriptor {
                    field, set, binding,
                    kind: make_kind_tokens(&kind_name),
                    stages,
                }))
            } else {
                Ok(None)
            }
        })?;

        // Validate uniqueness
        collect::unique_by(descriptors, |r| ((r.set, r.binding), r.span), "duplicate (set, binding)")
    }

    fn trait_path() -> TokenStream {
        quote! { macrokid_graphics::resources::ResourceBindings }
    }

    fn method_name() -> Ident {
        Ident::new("bindings", Span::call_site())
    }

    fn module_hint() -> &'static str { "rb" }
}
```

**User-facing API:**

```rust
#[derive(ResourceBinding)]
struct Material {
    #[uniform(set = 0, binding = 0, stages = "vs|fs")]
    matrices: Mat4,

    #[texture(set = 0, binding = 1, stages = "fs")]
    albedo: Texture2D,

    #[sampler(set = 0, binding = 2, stages = "fs")]
    sampler: Sampler,
}

// Generated automatically:
impl ResourceBindings for Material {
    fn bindings() -> &'static [BindingDesc] { /* ... */ }
}
```

---

### Example 2: Using Builder API for Dynamic Config

```rust
fn config_based_derive(config: &DeriveConfig) -> impl Fn(DeriveInput) -> syn::Result<TokenStream> {
    let builder = StaticSliceBuilder::new()
        .descriptor_type(config.descriptor_type.clone())
        .trait_path(config.trait_path.clone())
        .method_name(&config.method)
        .module_hint(&config.module)
        .collector(config.collector.clone());

    move |input| {
        let spec = TypeSpec::from_derive_input(input)?;
        builder.generate(&spec)
    }
}
```

---

### Example 3: Declarative DSL for Quick Derives

```rust
use macrokid_core::derive_item;

derive_item! {
    /// Pipeline configuration derive
    pub struct PipelineConfig {
        descriptor_type: PipelineDesc,
        trait_path: my_crate::PipelineInfo,
        method: pipeline_desc,
        module: pipeline,
        static_name: DESC,

        fn build(spec: &TypeSpec) -> syn::Result<PipelineDesc> {
            // Parse type-level attributes
            let attrs = parse_pipeline_attrs(spec)?;

            Ok(PipelineDesc {
                name: spec.ident.to_string(),
                vertex_shader: attrs.vs,
                fragment_shader: attrs.fs,
                topology: attrs.topology,
                // ...
            })
        }
    }
}
```

---

## Performance

All combinators are **zero-cost abstractions**:

- PhantomData-based (zero runtime size)
- Fully inlined at compile time
- No allocations or indirection
- Generated code is identical to hand-written

**Benchmark:** ResourceBinding derive generates identical ASM before/after refactoring.

---

## Comparison Table

| Approach | LoC | Flexibility | Boilerplate | Best For |
|----------|-----|-------------|-------------|----------|
| **Trait Impl** | ~60 | High | Medium | Complex derives, custom logic |
| **Builder API** | ~40 | Medium | Low | Dynamic config, runtime flexibility |
| **Declarative DSL** | ~15 | Low | Minimal | Quick derives, prototyping |

---

## Next Steps

1. **Read the source:**
   - `macrokid_core/src/common/gen.rs` - Combinators
   - `macrokid_core/src/common/derive_patterns.rs` - Patterns
   - `macrokid_core/src/common/derive_dsl.rs` - DSL macros

2. **Refactor existing derives:**
   - Start with BufferLayout (complex example)
   - Try GraphicsPipeline (StaticItemDerive)

3. **Build your own derive:**
   - Use the DSL for quick prototyping
   - Graduate to trait impl if you need more control

4. **Explore composability:**
   - Combine generators with `seq!` and `try_seq!`
   - Mix CodeGen and ResultCodeGen with adapters

---

## Tips & Tricks

### Tip 1: Start with the DSL

```rust
// Prototype fast with DSL
derive_slice! { /* ... */ }

// Refactor to trait impl only if needed
impl StaticSliceDerive for MyDerive { /* ... */ }
```

### Tip 2: Use `try_seq!` for Validation Pipelines

```rust
type SafeDerive = try_seq![
    ValidateInputGen,      // ✓ Check struct shape
    ParseAttributesGen,    // ✓ Parse and validate attrs
    CollectDescriptorsGen, // ✓ Build descriptors
    GenerateCodeGen        // ✓ Generate final tokens
];
```

### Tip 3: Compose Patterns

```rust
// Combine multiple patterns in one derive
type FullDerive = seq![
    StaticSlicePattern,   // Generate descriptor slice
    TraitImplPattern,     // Implement trait
    InherentPattern,      // Add convenience methods
    DebugPattern          // Optional debug impl
];
```

### Tip 4: Leverage Validation Helpers

```rust
use macrokid_core::common::derive_patterns::validation;

// Check uniqueness
validation::validate_unique(&records, |r| r.id, "duplicate ID")?;

// Check ranges
validation::validate_range(value, 0, 255, "value")?;
```

---

## Conclusion

The macrokid code generation framework gives you:

✅ **3 ways** to build derives (trait, builder, DSL)
✅ **Fallible composition** with ResultCodeGen
✅ **Zero-cost** abstractions
✅ **50-70% code reduction** in real derives
✅ **Type-safe** composition

**Start simple, scale up as needed!**

Happy macro hacking! 🚀
