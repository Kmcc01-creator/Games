# Code Generation Framework - Quick Reference

## Combinators (`gen.rs`)

### Infallible (CodeGen)

```rust
// Chain two generators
type Both = Chain<A, B>;

// Sequence multiple
type Many = seq![A, B, C, D];

// Conditional
type Branch = Conditional<Predicate, TrueGen, FalseGen>;

// Transform input
type Mapped = Map<Gen, MapFn>;

// Noop (empty)
Noop
```

### Fallible (ResultCodeGen)

```rust
// Chain with error propagation
type Both = ResultChain<A, B>;

// Sequence multiple fallible
type Many = try_seq![A, B, C, D];

// Conditional with fallible branches
type Branch = TryConditional<Predicate, TrueGen, FalseGen>;

// Fallible conditional
type Branch = ResultConditional<ResultPredicate, TrueGen, FalseGen>;
```

### Adapters

```rust
// Lift CodeGen → ResultCodeGen
type Lifted = Lift<MyGen>;

// Unwrap ResultCodeGen → CodeGen (panics on error)
type Unwrapped = Unwrap<MyGen>;
```

## Derive Patterns

### StaticSliceDerive (Field-based)

**Trait Implementation:**
```rust
struct MyDerive;

impl StaticSliceDerive for MyDerive {
    type Descriptor = MyDesc;
    fn descriptor_type() -> TokenStream { quote! { MyDesc } }
    fn collect_descriptors(spec: &TypeSpec) -> syn::Result<Vec<MyDesc>> { /* ... */ }
    fn trait_path() -> TokenStream { quote! { MyTrait } }
    fn method_name() -> Ident { ident("method") }
    fn module_hint() -> &'static str { "hint" }
}
```

**Builder API:**
```rust
StaticSliceBuilder::new()
    .descriptor_type(quote! { MyDesc })
    .trait_path(quote! { MyTrait })
    .method_name("method")
    .module_hint("hint")
    .collector(|spec| Ok(vec![/* ... */]))
    .generate(&spec)?
```

**Declarative DSL:**
```rust
derive_slice! {
    pub struct MyDerive {
        descriptor_type: MyDesc,
        trait_path: MyTrait,
        method: method,
        module: hint,
        fn collect(spec: &TypeSpec) -> syn::Result<Vec<MyDesc>> { /* ... */ }
    }
}
```

### StaticItemDerive (Single item)

**Trait Implementation:**
```rust
struct MyDerive;

impl StaticItemDerive for MyDerive {
    type Descriptor = MyDesc;
    fn descriptor_type() -> TokenStream { quote! { MyDesc } }
    fn build_descriptor(spec: &TypeSpec) -> syn::Result<MyDesc> { /* ... */ }
    fn trait_path() -> TokenStream { quote! { MyTrait } }
    fn method_name() -> Ident { ident("method") }
    fn module_hint() -> &'static str { "hint" }
}
```

**Builder API:**
```rust
StaticItemBuilder::new()
    .descriptor_type(quote! { MyDesc })
    .trait_path(quote! { MyTrait })
    .method_name("method")
    .module_hint("hint")
    .builder(|spec| Ok(MyDesc { /* ... */ }))
    .generate(&spec)?
```

**Declarative DSL:**
```rust
derive_item! {
    pub struct MyDerive {
        descriptor_type: MyDesc,
        trait_path: MyTrait,
        method: method,
        module: hint,
        fn build(spec: &TypeSpec) -> syn::Result<MyDesc> { /* ... */ }
    }
}
```

## Validation Helpers

```rust
use macrokid_core::common::derive_patterns::validation;

// Uniqueness
validation::validate_unique(&items, |i| i.key, "duplicate key")?;

// Range
validation::validate_range(value, 0, 100, "value")?;
```

## Common Patterns

### Validation Pipeline

```rust
type SafeDerive = try_seq![
    ValidateStructGen,   // Check input shape
    ParseAttrsGen,       // Parse attributes
    CollectGen,          // Collect descriptors
    GenerateGen          // Generate code
];

let result = <SafeDerive as ResultCodeGen<Input>>::generate(&input)?;
```

### Conditional by Type

```rust
struct IsEnum;
impl Predicate<TypeSpec> for IsEnum {
    fn test(spec: &TypeSpec) -> bool {
        matches!(spec.kind, TypeKind::Enum(_))
    }
}

type Adaptive = Conditional<IsEnum, EnumGen, StructGen>;
```

### Transform-Generate

```rust
struct ExtractFields;
impl MapFn<TypeSpec> for ExtractFields {
    type To = Vec<FieldSpec>;
    fn map(spec: &TypeSpec) -> Vec<FieldSpec> { spec.fields() }
}

type Mapped = Map<FieldOnlyGen, ExtractFields>;
```

## Imports

```rust
// Combinators
use macrokid_core::common::gen::{
    CodeGen, ResultCodeGen,
    Chain, ResultChain,
    Conditional, TryConditional,
    Map, Noop, Lift, Unwrap,
};
use macrokid_core::{seq, try_seq};

// Patterns
use macrokid_core::common::derive_patterns::{
    StaticSliceDerive, StaticItemDerive,
    StaticSliceBuilder, StaticItemBuilder,
    validation,
};

// DSL
use macrokid_core::{derive_slice, derive_item};

// IR & Utilities
use macrokid_core::ir::{TypeSpec, TypeKind, FieldSpec};
use macrokid_core::collect;
use macrokid_core::exclusive_schemas;
```

## Testing

```rust
cargo test -p macrokid_core --features codegen --lib gen
# 14 tests for combinators

cargo check -p macrokid_core --features codegen
# Verify compilation
```

## Key Files

| File | Purpose |
|------|---------|
| `macrokid_core/src/common/gen.rs` | Combinators & composition |
| `macrokid_core/src/common/derive_patterns.rs` | Pattern traits & builders |
| `macrokid_core/src/common/derive_dsl.rs` | Declarative macros |
| `macrokid_graphics_derive/src/lib.rs` | Real-world examples |

## Decision Matrix

| Need | Use |
|------|-----|
| Quick prototype | `derive_slice!` or `derive_item!` |
| Custom generation logic | Trait implementation |
| Dynamic configuration | Builder API |
| Error handling | `ResultCodeGen` + `try_seq!` |
| Conditional logic | `Conditional` or `TryConditional` |
| Input transformation | `Map<Gen, MapFn>` |
| Sequencing | `seq!` or `try_seq!` |

## Performance Notes

- All combinators are **zero-cost** (PhantomData-based)
- No runtime overhead vs hand-written code
- Fully inlined at compile time
- Generated code is identical

## Common Gotchas

1. **ResultCodeGen requires fully qualified syntax:**
   ```rust
   // ✗ Won't compile
   MyDerive::generate(&input)

   // ✓ Correct
   <MyDerive as ResultCodeGen<Input>>::generate(&input)?
   ```

2. **Descriptor must implement ToTokens:**
   ```rust
   impl ToTokens for MyDescriptor {
       fn to_tokens(&self, tokens: &mut TokenStream) { /* ... */ }
   }
   ```

3. **Builder collectors must be 'static:**
   ```rust
   .collector(|spec| { /* closure must be 'static */ })
   ```

## Cheat Sheet

```rust
// Quickest: DSL
derive_slice! { /* ... */ }

// Most flexible: Trait
impl StaticSliceDerive for MyDerive { /* ... */ }

// Most dynamic: Builder
StaticSliceBuilder::new()./* ... */.generate(&spec)?

// Fallible sequence
type Safe = try_seq![A, B, C];

// Conditional
type Branch = Conditional<Pred, TrueGen, FalseGen>;

// Transform
type Mapped = Map<Gen, MapFn>;
```

---

**Need help?** See `CODEGEN_TUTORIAL.md` for detailed examples!
