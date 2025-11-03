# Theoretical Foundations: Validation Combinators and Functional Completeness

> Mathematical underpinnings of zero-cost validation combinators in Rust

## 🧮 Boolean Algebra Foundations

### Basic Operations

Validation combinators directly correspond to boolean algebra operations:

| Boolean | Math Symbol | Combinator | Meaning |
|---------|-------------|------------|---------|
| AND | `A ∧ B` | `And<A, B>` | Both must be true |
| OR | `A ∨ B` | `Or<A, B>` | At least one must be true |  
| NOT | `¬A` | `Not<A>` | Must be false |
| XOR | `A ⊕ B` | `Xor<A, B>` | Exactly one must be true |
| IMPLIES | `A → B` | `Implies<A, B>` | If A then B |
| IFF | `A ↔ B` | `Iff<A, B>` | A if and only if B |

### De Morgan's Laws

These fundamental laws apply directly to our combinators:

```rust
// ¬(A ∧ B) = (¬A) ∨ (¬B)
type NotAndAB = Not<And<A, B>>;
type NotAOrNotB = Or<Not<A>, Not<B>>;
// These are logically equivalent

// ¬(A ∨ B) = (¬A) ∧ (¬B) 
type NotOrAB = Not<Or<A, B>>;
type NotAAndNotB = And<Not<A>, Not<B>>;
// These are logically equivalent
```

### Distributive Laws

```rust
// A ∧ (B ∨ C) = (A ∧ B) ∨ (A ∧ C)
type AAndBOrC = And<A, Or<B, C>>;
type AAndBOrAAndC = Or<And<A, B>, And<A, C>>;

// A ∨ (B ∧ C) = (A ∨ B) ∧ (A ∨ C)
type AOrBAndC = Or<A, And<B, C>>;
type AOrBAndAOrC = And<Or<A, B>, Or<A, C>>;
```

## 🔗 Functional Completeness

### Definition
A set of boolean operators is **functionally complete** if it can express every possible boolean function. 

### Complete Sets
- `{NAND}` - Single operator completeness
- `{NOR}` - Single operator completeness  
- `{AND, OR, NOT}` - Traditional complete set
- `{AND, NOT}` - Minimal traditional set
- `{OR, NOT}` - Alternative minimal set

### Proof: NAND Completeness

```rust
// NOT A = A NAND A
type Not<A> = Nand<A, A>;

// A AND B = NOT(A NAND B) = (A NAND B) NAND (A NAND B)
type And<A, B> = Nand<Nand<A, B>, Nand<A, B>>;

// A OR B = NOT A NAND NOT B = (A NAND A) NAND (B NAND B)
type Or<A, B> = Nand<Nand<A, A>, Nand<B, B>>;

// Since {AND, OR, NOT} is complete, and we can build all three from NAND,
// therefore NAND alone is functionally complete.
```

## 🎯 Type-Level Computation

### Church Encoding in Types

Our combinators represent a form of Church encoding at the type level:

```rust
// Church booleans as validators
struct True;   // Always passes
struct False;  // Always fails

// Church conditionals
type If<P, T, F> = Or<And<P, T>, And<Not<P>, F>>;
```

### Curry-Howard Correspondence

The Curry-Howard correspondence relates logic to type theory:

| Logic | Type Theory | Our System |
|-------|-------------|------------|
| Proposition | Type | Validator trait |
| Proof | Term/Value | Successful validation |
| Conjunction (∧) | Product type | `And<A, B>` |
| Disjunction (∨) | Sum type | `Or<A, B>` |
| Implication (→) | Function type | `Implies<A, B>` |
| Negation (¬) | Absurdity | `Not<A>` |

### Type-Level Recursion

```rust
// Recursive combinator definitions
type AllOf<List> = /* fold List with And */;
type AnyOf<List> = /* fold List with Or */;
type NoneOf<List> = Not<AnyOf<List>>;

// Example: All validators in a tuple must pass
impl<A, B, C> Validator<Cfg> for AllOf<(A, B, C)>
where A: Validator<Cfg>, B: Validator<Cfg>, C: Validator<Cfg>
{
    type Error = String; // Simplified
    fn validate(cfg: &Cfg) -> Result<(), String> {
        A::validate(cfg)?;
        B::validate(cfg)?;
        C::validate(cfg)?;
        Ok(())
    }
}
```

## 🧬 Algebraic Properties

### Commutativity
```rust
// A ∧ B = B ∧ A
And<A, B> ≡ And<B, A>  // Not enforced by type system, but semantically equivalent

// A ∨ B = B ∨ A  
Or<A, B> ≡ Or<B, A>
```

### Associativity
```rust
// (A ∧ B) ∧ C = A ∧ (B ∧ C)
And<And<A, B>, C> ≡ And<A, And<B, C>>

// (A ∨ B) ∨ C = A ∨ (B ∨ C)
Or<Or<A, B>, C> ≡ Or<A, Or<B, C>>
```

### Identity Elements
```rust
// A ∧ True = A
And<A, Always> ≡ A

// A ∨ False = A  
Or<A, Never> ≡ A

// A ∧ False = False
And<A, Never> ≡ Never

// A ∨ True = True
Or<A, Always> ≡ Always
```

### Idempotency
```rust
// A ∧ A = A
And<A, A> ≡ A

// A ∨ A = A
Or<A, A> ≡ A
```

## 🔄 Optimization Theory

### Normal Forms

**Disjunctive Normal Form (DNF)**
Every boolean expression can be written as OR of ANDs:
```rust
type DNF = Or<
    And<A, And<B, C>>,
    Or<
        And<D, Not<E>>,
        And<F, And<G, H>>
    >
>;
```

**Conjunctive Normal Form (CNF)**  
Every boolean expression can be written as AND of ORs:
```rust
type CNF = And<
    Or<A, Or<B, C>>,
    And<
        Or<D, Not<E>>,
        Or<F, Or<G, H>>
    >
>;
```

### Compiler Optimizations

The Rust compiler can optimize combinator expressions:

```rust
// Input: Redundant validation
type Redundant = And<A, And<A, B>>;

// Optimized: A appears only once  
type Optimized = And<A, B>;

// Input: Contradiction
type Impossible = And<A, Not<A>>;

// Optimized: Always fails
type OptimizedImpossible = Never;

// Input: Tautology
type Tautology = Or<A, Not<A>>;

// Optimized: Always passes
type OptimizedTautology = Always;
```

## 🌐 Category Theory Connections

### Monoid Structure

Validation combinators form monoids under certain operations:

```rust
// (And, Always) forms a monoid
// Identity: Always
// Associative: And<And<A, B>, C> ≡ And<A, And<B, C>>

// (Or, Never) forms a monoid  
// Identity: Never
// Associative: Or<Or<A, B>, C> ≡ Or<A, Or<B, C>>
```

### Lattice Structure

Validators form a Boolean lattice:
- **Join (∨)**: `Or<A, B>` (least upper bound)
- **Meet (∧)**: `And<A, B>` (greatest lower bound)  
- **Top (⊤)**: `Always` (accepts everything)
- **Bottom (⊥)**: `Never` (rejects everything)
- **Complement (¬)**: `Not<A>` (logical negation)

## 🔬 Complexity Analysis

### Time Complexity
- **Simple combinators**: O(1) - compile to direct calls
- **Complex nesting**: O(depth) - limited by short-circuiting
- **Worst case**: O(n) where n is number of primitive validators

### Space Complexity
- **Runtime**: O(0) - all combinators are zero-sized types
- **Compile-time**: O(depth) - type resolution stack depth
- **Generated code**: O(n) - proportional to number of actual checks

### Type Resolution Complexity
```rust
// Linear in depth
type Depth3 = And<And<A, B>, C>;              // O(3)
type Depth5 = And<And<And<And<A, B>, C>, D>, E>; // O(5)

// Better: Use type aliases to manage complexity
type Layer1 = And<A, B>;
type Layer2 = And<Layer1, C>;
type Layer3 = And<Layer2, D>;  // Still O(5) but easier for compiler
```

## 📐 Formal Verification

### Satisfiability (SAT)

Each combinator expression is essentially a SAT formula:

```rust
type Formula = And<Or<A, B>, And<C, Not<D>>>;

// Corresponding SAT formula: (A ∨ B) ∧ C ∧ ¬D
// Can use SAT solvers to:
// 1. Check if formula is satisfiable
// 2. Find satisfying assignments  
// 3. Optimize formulas
// 4. Detect contradictions
```

### Model Checking

Validation combinators can be used for formal verification:

```rust
// System invariant: "If in debug mode, validation must be enabled"
type SystemInvariant = Implies<DebugMode, ValidationEnabled>;

// Safety property: "Never have both high performance and debug enabled"  
type SafetyProperty = Not<And<HighPerformance, DebugMode>>;

// Liveness property: "Eventually either GPU validation passes or fallback is used"
type LivenessProperty = Or<GPUValidation, FallbackMode>;
```

## 🎓 Related Mathematical Concepts

### Boolean Rings
```rust
// XOR and AND form a boolean ring structure
// XOR is addition, AND is multiplication
type Add<A, B> = Xor<A, B>;
type Mul<A, B> = And<A, B>;

// Ring axioms hold:
// Additive identity: Never (false)
// Multiplicative identity: Always (true)
// Distributivity: A ∧ (B ⊕ C) = (A ∧ B) ⊕ (A ∧ C)
```

### Stone's Representation Theorem
Every Boolean algebra is isomorphic to a field of sets. Our validation combinators represent this field of sets where each validator corresponds to a set of valid configurations.

### Lindenbaum-Tarski Algebra
The quotient of validation formulas under logical equivalence forms a Boolean algebra, which is precisely what our combinator system represents.

## 🔮 Future Research Directions

### Quantum Logic
```rust
// Non-commutative validation for quantum-inspired systems
struct QuantumAnd<A, B>(PhantomData<(A, B)>);
// A ∧ B ≠ B ∧ A in quantum logic
```

### Fuzzy Logic
```rust
// Validation with degrees of truth
trait FuzzyValidator<Cfg> {
    fn validate_fuzzy(cfg: &Cfg) -> f64; // Returns value in [0, 1]
}
```

### Temporal Logic
```rust
// Validation over time sequences
trait TemporalValidator<Cfg> {
    fn validate_always(&self, history: &[Cfg]) -> bool;
    fn validate_eventually(&self, history: &[Cfg]) -> bool;
    fn validate_until<Other>(&self, other: &Other, history: &[Cfg]) -> bool;
}
```

## 📚 Bibliography

- **Boolean Algebra**: George Boole, "An Investigation of the Laws of Thought" (1854)
- **Functional Completeness**: Emil Post, "Introduction to a general theory of elementary propositions" (1921)
- **Curry-Howard Correspondence**: Haskell Curry, William Howard (1940s-1960s)
- **Stone Duality**: Marshall Stone, "The theory of representations for Boolean algebras" (1936)
- **SAT Solving**: Martin Davis, Hilary Putnam, "A computing procedure for quantification theory" (1960)

---

*This theoretical foundation demonstrates that validation combinators are not just a practical programming technique, but represent a rigorous mathematical framework with deep connections to logic, algebra, and computer science theory.*# Validation Combinator Implementation Guide

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

This implementation guide provides everything needed to extend macrokid_core with functionally complete validation combinators while maintaining zero runtime cost and maximum composability.# Functional Completeness in Validation Combinators

> "Just as NAND gates can build any digital circuit, a minimal set of validation combinators can express any validation logic."

## 🎯 Core Insight

In digital electronics, **NAND** and **NOR** gates are "functionally complete" - you can build any logical circuit using just one of these gate types. This same principle applies to validation combinators in macrokid_core.

**Key Discovery**: With just 3 primitive combinators (`And`, `Or`, `Not`), we can express arbitrarily complex validation logic at **zero runtime cost**.

## 🧠 Theoretical Foundation

### Boolean Algebra → Validation Logic

| Boolean Logic | Validation Combinator | Meaning |
|---------------|----------------------|---------|
| `A ∧ B` | `And<A, B>` | Both validators must pass |
| `A ∨ B` | `Or<A, B>` | Either validator can pass |
| `¬A` | `Not<A>` | Validator must fail |
| `A ⊕ B` | `Xor<A, B>` | Exactly one validator passes |
| `A → B` | `Implies<A, B>` | If A passes, then B must pass |

### The Universal Set

From these three primitives:
```rust
pub struct And<A, B>(PhantomData<(A, B)>);
pub struct Or<A, B>(PhantomData<(A, B)>);  
pub struct Not<A>(PhantomData<A>);
```

We can construct **any validation logic**:

```rust
// XOR: (A OR B) AND NOT(A AND B)
type Xor<A, B> = And<Or<A, B>, Not<And<A, B>>>;

// IMPLIES: A → B = (NOT A) OR B  
type Implies<A, B> = Or<Not<A>, B>;

// IF-THEN-ELSE: IF A THEN B ELSE C = (A AND B) OR (NOT A AND C)
type IfThenElse<A, B, C> = Or<And<A, B>, And<Not<A>, C>>;

// NAND: NOT(A AND B)
type Nand<A, B> = Not<And<A, B>>;

// NOR: NOT(A OR B)  
type Nor<A, B> = Not<Or<A, B>>;
```

## 🔬 The NAND Approach

For maximum theoretical elegance, we could implement **everything** using just NAND:

```rust
/// Universal validation combinator - functionally complete on its own
pub struct Nand<A, B>(PhantomData<(A, B)>);

impl<Cfg, A, B, E> Validator<Cfg> for Nand<A, B>
where
    A: Validator<Cfg, Error = E>,
    B: Validator<Cfg, Error = E>,
    E: Default,
{
    type Error = E;
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        let a_result = A::validate(cfg);
        let b_result = B::validate(cfg);
        
        match (a_result, b_result) {
            (Ok(()), Ok(())) => Err(E::default()), // Both pass = NAND fails
            _ => Ok(()), // At least one failed = NAND passes
        }
    }
}

// Build everything from NAND:
type Not<A> = Nand<A, A>;                           // A NAND A
type And<A, B> = Nand<Nand<A, B>, Nand<A, B>>;     // NOT(A NAND B)
type Or<A, B> = Nand<Nand<A, A>, Nand<B, B>>;      // (NOT A) NAND (NOT B)
```

## 🏗️ Practical Implementation

### Core Combinator Set

```rust
// macrokid_core/src/common/validate.rs

/// Both validators must pass (logical AND)
pub struct And<A, B>(PhantomData<(A, B)>);

/// Either validator can pass (logical OR)  
pub struct Or<A, B>(PhantomData<(A, B)>);

/// Validator must fail (logical NOT)
pub struct Not<A>(PhantomData<A>);

/// Always passes (logical TRUE)
pub struct Always;

/// Always fails (logical FALSE)
pub struct Never;
```

### Implementation Pattern

```rust
impl<Cfg, A, B, E> Validator<Cfg> for And<A, B>
where
    A: Validator<Cfg, Error = E>,
    B: Validator<Cfg, Error = E>,
{
    type Error = E;
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        A::validate(cfg)?;  // Short-circuit on failure
        B::validate(cfg)?;
        Ok(())
    }
}

impl<Cfg, A, B, E> Validator<Cfg> for Or<A, B>
where
    A: Validator<Cfg, Error = E>,
    B: Validator<Cfg, Error = E>,
{
    type Error = E;
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        A::validate(cfg)
            .or_else(|_| B::validate(cfg))  // Try B if A fails
    }
}

impl<Cfg, A> Validator<Cfg> for Not<A>
where
    A: Validator<Cfg>,
{
    type Error = String;  // Simplified for example
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        match A::validate(cfg) {
            Ok(()) => Err("Expected validation to fail".to_string()),
            Err(_) => Ok(()),
        }
    }
}
```

## 🎮 Real-World Examples

### Graphics Pipeline Validation

```rust
// Complex graphics validation using combinator logic
type GraphicsValidation = And<
    // Modern GPU with API support
    And<IsModernGPU, Or<SupportsVulkan, SupportsDirectX12>>,
    // Performance requirements  
    Or<
        HighEndHardware,
        And<MediumHardware, OptimizationsEnabled>
    >
>;

// Debug-specific validation: "If debug mode, then validation layers"
type DebugValidation = Or<Not<DebugMode>, ValidationLayersEnabled>;

// Resource binding: "Texture XOR uniform buffer"
type ResourceValidation = And<
    Or<TextureBinding, UniformBinding>,
    Not<And<TextureBinding, UniformBinding>>  // XOR logic
>;

// Final pipeline validation combines all constraints
type FullPipelineValidation = And<
    GraphicsValidation,
    And<DebugValidation, ResourceValidation>
>;

// Usage
fn validate_pipeline(config: &PipelineConfig) -> Result<(), ValidationError> {
    config.validate_with::<FullPipelineValidation>()?;
    Ok(())
}
```

### Macro Generation Logic

```rust
// Code generation decisions as validation logic
type ShouldGenerateGetters = And<
    IsStruct,
    Or<HasPublicFields, HasGetterAttribute>
>;

type ShouldGenerateDisplay = Or<
    HasDisplayAttribute,
    And<IsEnum, Not<HasComplexVariants>>
>;

// Conditional code generation
type CodeGenStrategy = IfThenElse<
    IsEnum,
    EnumCodeGenerator,
    StructCodeGenerator
>;
```

## 🚀 Advanced Applications

### 1. Macro Syntax for Complex Logic

```rust
// Hypothetical macro syntax for validation logic
validation_logic! {
    graphics_validation = (modern_gpu AND (vulkan OR directx12)) 
                         AND (high_end OR (medium AND optimized));
    
    debug_validation = NOT debug_mode OR validation_layers;
    
    full_validation = graphics_validation AND debug_validation;
}

// Expands to the combinator types above
```

### 2. Circuit-Style Visualization

```
[Modern GPU] ──┐
               ├─ AND ──┐
[Vulkan|DX12] ─┘       ├─ AND ── [Graphics Valid]
                       │
[High End] ────────────┼────────┘
[Medium] ─┐            │
          ├─ AND ──────┘
[Optimized]─┘

[Debug] ─── NOT ──┐
                  ├─ OR ── [Debug Valid]
[Val Layers] ─────┘
```

### 3. Compile-Time Optimization

```rust
// The type system could optimize these automatically:

// Redundant: A AND A → A
type Redundant<A> = And<A, A>;
type Optimized<A> = A;

// Contradiction: A AND NOT A → Never
type Impossible<A> = And<A, Not<A>>;
type OptimizedImpossible = Never;

// Tautology: A OR NOT A → Always  
type Tautology<A> = Or<A, Not<A>>;
type OptimizedTautology = Always;
```

## 📊 Performance Characteristics

### Zero Runtime Cost
```rust
// All combinator types are zero-sized
assert_eq!(std::mem::size_of::<And<A, B>>(), 0);
assert_eq!(std::mem::size_of::<Or<A, B>>(), 0);
assert_eq!(std::mem::size_of::<Not<A>>(), 0);

// Even complex combinations
type ComplexValidation = And<Or<A, B>, Not<And<C, D>>>;
assert_eq!(std::mem::size_of::<ComplexValidation>(), 0);
```

### Compile-Time Expansion
```rust
// This complex type expression:
config.validate_with::<And<Or<A, B>, Not<C>>>()?;

// Compiles to simple function calls:
A::validate(config)
    .or_else(|_| B::validate(config))?;
if C::validate(config).is_ok() {
    return Err("C should not pass".into());
}
```

## 🎯 Design Principles

### 1. Minimal Core, Maximum Power
- Ship with just 3-4 primitive combinators
- Users compose everything else
- No feature bloat in core library

### 2. Type-Driven Design
- Complex logic expressed as type relationships
- Compiler catches logic errors
- Zero runtime abstraction penalty

### 3. Composability First
- Every combinator works with every other
- No special cases or exceptions
- Infinite nesting possible

### 4. Mathematical Foundation
- Based on proven boolean algebra
- Predictable semantics
- Optimizable by formal methods

## 🔮 Future Possibilities

### Visual Validation Designer
- Drag-and-drop logic gate interface
- Generates combinator type expressions
- Live validation testing

### Macro-Generated Combinators
```rust
combinator! {
    /// Validates that exactly N out of M conditions pass
    NOutOfM<const N: usize, T: Tuple> = /* generated logic */;
}
```

### Cross-Language Compilation
- Combinator expressions → SQL WHERE clauses
- Combinator expressions → JavaScript validation
- Combinator expressions → GraphQL schema validation

## 📚 Related Work

- **Boolean Satisfiability (SAT)**: Our combinator expressions are essentially SAT formulas
- **Type-Level Programming**: Similar to type-level computations in Haskell/Scala
- **Parser Combinators**: Same composability principles applied to parsing
- **Functional Programming**: Monads and applicative functors for validation

## 🎉 Conclusion

By applying functional completeness principles from digital logic to validation combinators, we've created a **universal validation language** that is:

- ✅ **Theoretically complete**: Can express any validation logic
- ✅ **Zero runtime cost**: Pure compile-time abstraction
- ✅ **Infinitely composable**: No limits to complexity
- ✅ **Type safe**: Compiler catches errors
- ✅ **Mathematically founded**: Based on boolean algebra

This transforms validation from **imperative code** to **declarative type expressions**, making complex validation logic both more reliable and more maintainable.

---

*"In the same way that NAND gates revolutionized digital circuit design by providing a universal building block, validation combinators provide a universal building block for expressing complex validation logic in type-safe, zero-cost abstractions."*