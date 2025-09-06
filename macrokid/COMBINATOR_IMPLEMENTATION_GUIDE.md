# Validation Combinator Implementation Guide

> Step-by-step guide for implementing functionally complete validation combinators in macrokid_core

## 🎯 Goal

Extend `macrokid_core::common::validate` with a minimal set of combinators that can express any validation logic at zero runtime cost.

## 📦 What We're Building

Starting from your current `And<A, B>` combinator, we'll add:
- `Or<A, B>` - Alternative validation (either passes)
- `Not<A>` - Negation validation (must fail)
- Helper combinators built from these primitives

## 🚀 Step 1: Add Core Combinators

### Add to `macrokid_core/src/common/validate.rs`

```rust
/// Either validator can pass (logical OR)
/// Tries A first, then B if A fails
pub struct Or<A, B>(core::marker::PhantomData<(A, B)>);

impl<Cfg, A, B, E> Validator<Cfg> for Or<A, B>
where
    A: Validator<Cfg, Error = E>,
    B: Validator<Cfg, Error = E>,
{
    type Error = E;
    
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        // Try A first
        match A::validate(cfg) {
            Ok(()) => Ok(()),
            Err(_) => B::validate(cfg), // If A fails, try B
        }
    }
}

/// Validator must fail (logical NOT)
/// Succeeds if the inner validator fails
pub struct Not<A>(core::marker::PhantomData<A>);

impl<Cfg, A> Validator<Cfg> for Not<A>
where
    A: Validator<Cfg, Error = String>, // Simplified - could be generic
{
    type Error = String;
    
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        match A::validate(cfg) {
            Ok(()) => Err("Expected validation to fail, but it passed".to_string()),
            Err(_) => Ok(()), // Inner validation failed = NOT succeeds
        }
    }
}

/// Always passes validation (logical TRUE)
pub struct Always;

impl<Cfg> Validator<Cfg> for Always {
    type Error = String;
    
    fn validate(_cfg: &Cfg) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Always fails validation (logical FALSE)  
pub struct Never;

impl<Cfg> Validator<Cfg> for Never {
    type Error = String;
    
    fn validate(_cfg: &Cfg) -> Result<(), Self::Error> {
        Err("Never validator always fails".to_string())
    }
}
```

## 🔧 Step 2: Add Convenience Type Aliases

```rust
/// Exclusive OR - exactly one validator must pass
pub type Xor<A, B> = And<Or<A, B>, Not<And<A, B>>>;

/// Logical implication - if A passes, then B must pass  
pub type Implies<A, B> = Or<Not<A>, B>;

/// Optional validation - never fails but logs results
pub type Optional<A> = Or<A, Always>;

/// Conditional validation - if P passes use T, else use F
pub type IfThenElse<P, T, F> = Or<And<P, T>, And<Not<P>, F>>;
```

## 🧪 Step 3: Add Comprehensive Tests

```rust
#[cfg(test)]
mod combinator_tests {
    use super::*;

    // Test data
    struct TestConfig {
        value: i32,
        flag: bool,
    }

    // Test validators
    struct Positive;
    impl Validator<TestConfig> for Positive {
        type Error = String;
        fn validate(cfg: &TestConfig) -> Result<(), Self::Error> {
            if cfg.value > 0 { Ok(()) } else { Err("not positive".into()) }
        }
    }

    struct FlagSet;
    impl Validator<TestConfig> for FlagSet {
        type Error = String;
        fn validate(cfg: &TestConfig) -> Result<(), Self::Error> {
            if cfg.flag { Ok(()) } else { Err("flag not set".into()) }
        }
    }

    struct Even;
    impl Validator<TestConfig> for Even {
        type Error = String;
        fn validate(cfg: &TestConfig) -> Result<(), Self::Error> {
            if cfg.value % 2 == 0 { Ok(()) } else { Err("not even".into()) }
        }
    }

    #[test]
    fn test_or_both_pass() {
        let cfg = TestConfig { value: 4, flag: true }; // Both positive and even
        assert!(cfg.validate_with::<Or<Positive, Even>>().is_ok());
    }

    #[test] 
    fn test_or_first_passes() {
        let cfg = TestConfig { value: 3, flag: false }; // Positive but not even
        assert!(cfg.validate_with::<Or<Positive, Even>>().is_ok());
    }

    #[test]
    fn test_or_second_passes() {
        let cfg = TestConfig { value: -2, flag: false }; // Even but not positive
        assert!(cfg.validate_with::<Or<Positive, Even>>().is_ok());
    }

    #[test]
    fn test_or_both_fail() {
        let cfg = TestConfig { value: -3, flag: false }; // Neither positive nor even
        assert!(cfg.validate_with::<Or<Positive, Even>>().is_err());
    }

    #[test]
    fn test_not_passes() {
        let cfg = TestConfig { value: -1, flag: false }; // Not positive
        assert!(cfg.validate_with::<Not<Positive>>().is_ok());
    }

    #[test] 
    fn test_not_fails() {
        let cfg = TestConfig { value: 1, flag: false }; // Positive
        assert!(cfg.validate_with::<Not<Positive>>().is_err());
    }

    #[test]
    fn test_xor_exclusive() {
        // XOR should pass when exactly one condition is true
        let cfg1 = TestConfig { value: 3, flag: false }; // Positive but not even
        let cfg2 = TestConfig { value: -2, flag: false }; // Even but not positive  
        let cfg3 = TestConfig { value: 4, flag: false }; // Both positive and even (should fail XOR)
        let cfg4 = TestConfig { value: -3, flag: false }; // Neither (should fail XOR)

        assert!(cfg1.validate_with::<Xor<Positive, Even>>().is_ok());
        assert!(cfg2.validate_with::<Xor<Positive, Even>>().is_ok());
        assert!(cfg3.validate_with::<Xor<Positive, Even>>().is_err());
        assert!(cfg4.validate_with::<Xor<Positive, Even>>().is_err());
    }

    #[test]
    fn test_implies() {
        // If flag is set, then value must be positive
        type FlagImpliesPositive = Implies<FlagSet, Positive>;

        let cfg1 = TestConfig { value: 5, flag: true };   // Flag set, positive - OK
        let cfg2 = TestConfig { value: -5, flag: true };  // Flag set, negative - FAIL
        let cfg3 = TestConfig { value: -5, flag: false }; // Flag not set - OK (implication vacuously true)

        assert!(cfg1.validate_with::<FlagImpliesPositive>().is_ok());
        assert!(cfg2.validate_with::<FlagImpliesPositive>().is_err());
        assert!(cfg3.validate_with::<FlagImpliesPositive>().is_ok());
    }

    #[test]
    fn test_complex_expression() {
        // (Positive OR Even) AND NOT(Flag AND Positive)
        type Complex = And<Or<Positive, Even>, Not<And<FlagSet, Positive>>>;

        let cfg1 = TestConfig { value: 2, flag: false };  // Even, flag not set - OK
        let cfg2 = TestConfig { value: 3, flag: false };  // Positive, flag not set - OK  
        let cfg3 = TestConfig { value: 3, flag: true };   // Positive AND flag set - FAIL

        assert!(cfg1.validate_with::<Complex>().is_ok());
        assert!(cfg2.validate_with::<Complex>().is_ok());
        assert!(cfg3.validate_with::<Complex>().is_err());
    }

    #[test]
    fn test_zero_size() {
        // All combinators should be zero-sized
        assert_eq!(std::mem::size_of::<And<Positive, Even>>(), 0);
        assert_eq!(std::mem::size_of::<Or<Positive, Even>>(), 0);
        assert_eq!(std::mem::size_of::<Not<Positive>>(), 0);
        assert_eq!(std::mem::size_of::<Xor<Positive, Even>>(), 0);
        assert_eq!(std::mem::size_of::<Always>(), 0);
        assert_eq!(std::mem::size_of::<Never>(), 0);
    }
}
```

## 📋 Step 4: Usage Examples

### Graphics Pipeline Validation

```rust
// File: examples/graphics_validation.rs

use macrokid_core::common::validate::{ValidateExt, Validator, And, Or, Not};

struct GraphicsConfig {
    gpu_tier: GpuTier,
    api: GraphicsAPI, 
    debug_mode: bool,
    validation_layers: bool,
    render_passes: Vec<RenderPass>,
}

#[derive(PartialEq)]
enum GpuTier { Low, Medium, High }

#[derive(PartialEq)]  
enum GraphicsAPI { Vulkan, DirectX12, DirectX11, OpenGL }

struct RenderPass { /* ... */ }

// Individual validators
struct ModernGPU;
impl Validator<GraphicsConfig> for ModernGPU {
    type Error = String;
    fn validate(cfg: &GraphicsConfig) -> Result<(), String> {
        match cfg.gpu_tier {
            GpuTier::High | GpuTier::Medium => Ok(()),
            GpuTier::Low => Err("Modern GPU required".into()),
        }
    }
}

struct SupportsVulkan;
impl Validator<GraphicsConfig> for SupportsVulkan {
    type Error = String;
    fn validate(cfg: &GraphicsConfig) -> Result<(), String> {
        match cfg.api {
            GraphicsAPI::Vulkan => Ok(()),
            _ => Err("Vulkan not supported".into()),
        }
    }
}

struct SupportsDirectX12;  
impl Validator<GraphicsConfig> for SupportsDirectX12 {
    type Error = String;
    fn validate(cfg: &GraphicsConfig) -> Result<(), String> {
        match cfg.api {
            GraphicsAPI::DirectX12 => Ok(()),
            _ => Err("DirectX 12 not supported".into()),
        }
    }
}

struct DebugMode;
impl Validator<GraphicsConfig> for DebugMode {
    type Error = String;
    fn validate(cfg: &GraphicsConfig) -> Result<(), String> {
        if cfg.debug_mode { Ok(()) } else { Err("Not in debug mode".into()) }
    }
}

struct ValidationLayersEnabled;
impl Validator<GraphicsConfig> for ValidationLayersEnabled {
    type Error = String;
    fn validate(cfg: &GraphicsConfig) -> Result<(), String> {
        if cfg.validation_layers { Ok(()) } else { Err("Validation layers not enabled".into()) }
    }
}

// Complex validation logic using combinators
type ModernAPIValidation = And<ModernGPU, Or<SupportsVulkan, SupportsDirectX12>>;
type DebugValidation = Implies<DebugMode, ValidationLayersEnabled>;
type FullValidation = And<ModernAPIValidation, DebugValidation>;

fn validate_graphics_setup(config: &GraphicsConfig) -> Result<(), String> {
    config.validate_with::<FullValidation>()?;
    println!("Graphics configuration is valid!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_high_end_vulkan() {
        let config = GraphicsConfig {
            gpu_tier: GpuTier::High,
            api: GraphicsAPI::Vulkan,
            debug_mode: false,
            validation_layers: false,
            render_passes: vec![],
        };
        assert!(validate_graphics_setup(&config).is_ok());
    }

    #[test] 
    fn test_debug_requires_validation() {
        let config = GraphicsConfig {
            gpu_tier: GpuTier::High,
            api: GraphicsAPI::Vulkan,
            debug_mode: true,
            validation_layers: false, // Should fail - debug mode without validation
            render_passes: vec![],
        };
        assert!(validate_graphics_setup(&config).is_err());
    }

    #[test]
    fn test_low_gpu_fails() {
        let config = GraphicsConfig {
            gpu_tier: GpuTier::Low,
            api: GraphicsAPI::Vulkan,
            debug_mode: false,
            validation_layers: false,
            render_passes: vec![],
        };
        assert!(validate_graphics_setup(&config).is_err());
    }
}
```

## 🎨 Step 5: Macro Integration Example

```rust
// File: examples/macro_with_validation.rs

use macrokid_core::{
    derive_entry,
    common::validate::{ValidateExt, And, Or, Not},
    ir::{TypeSpec, TypeKind},
};

derive_entry!(ValidatedDisplay, handler = expand_validated_display);

fn expand_validated_display(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;

    // Define validation rules using combinators
    type StructWithFields = And<IsStruct, HasNamedFields>;
    type EnumWithVariants = And<IsEnum, HasVariants>;
    type ValidForDisplay = Or<StructWithFields, EnumWithVariants>;

    // Validate the input using our combinator
    spec.validate_with::<ValidForDisplay>()?;

    // Generate code based on validation result
    let impl_code = match &spec.kind {
        TypeKind::Struct(_) => generate_struct_display(&spec),
        TypeKind::Enum(_) => generate_enum_display(&spec),
    };

    Ok(impl_code)
}

// Validator implementations
struct IsStruct;
impl macrokid_core::common::validate::Validator<TypeSpec> for IsStruct {
    type Error = syn::Error;
    fn validate(spec: &TypeSpec) -> Result<(), Self::Error> {
        if spec.is_struct() {
            Ok(())
        } else {
            Err(syn::Error::new(spec.span, "Expected struct"))
        }
    }
}

struct IsEnum;
impl macrokid_core::common::validate::Validator<TypeSpec> for IsEnum {
    type Error = syn::Error;
    fn validate(spec: &TypeSpec) -> Result<(), Self::Error> {
        if spec.is_enum() {
            Ok(())
        } else {
            Err(syn::Error::new(spec.span, "Expected enum"))
        }
    }
}

struct HasNamedFields;
impl macrokid_core::common::validate::Validator<TypeSpec> for HasNamedFields {
    type Error = syn::Error;
    fn validate(spec: &TypeSpec) -> Result<(), Self::Error> {
        match &spec.kind {
            TypeKind::Struct(s) => match s.fields() {
                macrokid_core::FieldKind::Named(_) => Ok(()),
                _ => Err(syn::Error::new(spec.span, "Expected named fields")),
            },
            _ => Err(syn::Error::new(spec.span, "Not a struct")),
        }
    }
}

struct HasVariants;
impl macrokid_core::common::validate::Validator<TypeSpec> for HasVariants {
    type Error = syn::Error;
    fn validate(spec: &TypeSpec) -> Result<(), Self::Error> {
        match &spec.kind {
            TypeKind::Enum(e) => {
                if e.variants().is_empty() {
                    Err(syn::Error::new(spec.span, "Enum must have variants"))
                } else {
                    Ok(())
                }
            }
            _ => Err(syn::Error::new(spec.span, "Not an enum")),
        }
    }
}

fn generate_struct_display(spec: &TypeSpec) -> proc_macro2::TokenStream {
    // Implementation for struct Display generation
    quote::quote! { /* struct display impl */ }
}

fn generate_enum_display(spec: &TypeSpec) -> proc_macro2::TokenStream {
    // Implementation for enum Display generation  
    quote::quote! { /* enum display impl */ }
}
```

## ⚡ Performance Notes

### Compile-Time Verification

```bash
# Verify zero runtime cost
cargo test --release
cargo asm --release validate_with_complex_combinator
# Should show direct function calls, no combinator overhead
```

### Type Complexity Limits

```rust
// Rust has recursion limits for type resolution
// Very deep nesting might hit limits:
type VeryDeep = And<And<And<And<A, B>, C>, D>, E>; // OK
type TooDeep = /* 128+ levels of nesting */;        // May fail to compile

// Use type aliases to manage complexity:
type Layer1 = And<A, B>;
type Layer2 = And<Layer1, C>;
type Layer3 = And<Layer2, D>; // Much better
```

## 🔧 Integration Checklist

- [ ] Add `Or<A, B>` combinator to `validate.rs`
- [ ] Add `Not<A>` combinator to `validate.rs`  
- [ ] Add `Always` and `Never` primitives
- [ ] Add type aliases for common patterns (`Xor`, `Implies`, etc.)
- [ ] Write comprehensive test suite
- [ ] Update documentation with examples
- [ ] Add performance benchmarks
- [ ] Create usage examples for graphics/macro domains

## 🎯 Next Steps

1. **Implement the core combinators** following this guide
2. **Test with real validation scenarios** in your graphics code
3. **Measure compile-time impact** of complex combinator expressions
4. **Create domain-specific validator libraries** that use these primitives
5. **Consider macro syntax** for generating combinator expressions

## 🚀 Future Extensions

### Async Validation
```rust
pub trait AsyncValidator<Cfg> {
    type Error;
    async fn validate(cfg: &Cfg) -> Result<(), Self::Error>;
}

// Async combinators would work the same way
impl<Cfg, A, B, E> AsyncValidator<Cfg> for And<A, B> { /* ... */ }
```

### Streaming Validation  
```rust
pub trait StreamValidator<Item> {
    type Error;
    fn validate_stream<S: Stream<Item = Item>>(stream: S) -> impl Stream<Item = Result<Item, Self::Error>>;
}
```

### Cross-Language Targets
```rust
// Generate validation logic for other languages
trait ValidatorCodegen {
    fn to_sql_where_clause(&self) -> String;
    fn to_javascript(&self) -> String;
    fn to_json_schema(&self) -> serde_json::Value;
}
```

---

This implementation guide provides everything needed to extend macrokid_core with functionally complete validation combinators while maintaining zero runtime cost and maximum composability.