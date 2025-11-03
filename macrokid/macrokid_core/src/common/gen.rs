//! Composable, zero-cost code generation combinators (feature: `codegen`).
//!
//! Why
//! - Derive macros often need to “assemble” output from multiple small parts:
//!   emit static slices, add trait impls, add inherent helpers, etc. These parts are
//!   already easy to implement with `ImplBuilder`, `MatchArmBuilder`, and `common::codegen`.
//! - CodeGen provides a lightweight way to compose those parts declaratively and reuse them.
//! - All combinator types are zero-sized (PhantomData) and fully inlined at compile time.
//!
//! Core idea
//! - Write small, focused generators that implement `CodeGen<Input>`.
//! - Compose them with `Chain` and `Conditional` to build complex outputs.
//! - Use `Predicate<Input>` to branch by IR shape (e.g., enum vs struct).
//!
//! Abstract cases where it fits
//! 1) Conditional display generation
//!    - If enum: build per-variant arms via `MatchArmBuilder`; if struct: emit type-name display.
//! 2) Static data + helper methods
//!    - First emit `static_slice_mod` module; then add inherent methods to expose `DATA`.
//! 3) Config-driven toggles
//!    - Choose a generator based on a predicate (e.g., has attributes vs none).
//!
//! Example
//! ```ignore
//! use macrokid_core::common::gen::{CodeGen, Predicate, Chain, Conditional};
//! use macrokid_core::ir::TypeSpec;
//! use quote::quote;
//!
//! struct IsEnum;
//! impl Predicate<TypeSpec> for IsEnum { fn test(i: &TypeSpec) -> bool { i.is_enum() } }
//!
//! struct EnumDisplay;
//! impl CodeGen<TypeSpec> for EnumDisplay {
//!     type Output = proc_macro2::TokenStream;
//!     fn generate(spec: &TypeSpec) -> Self::Output {
//!         // build per-variant arms ...
//!         quote! { /* impl Display for enum */ }
//!     }
//! }
//!
//! struct StructDisplay;
//! impl CodeGen<TypeSpec> for StructDisplay {
//!     type Output = proc_macro2::TokenStream;
//!     fn generate(spec: &TypeSpec) -> Self::Output {
//!         // impl that writes the type name
//!         quote! { /* impl Display for struct */ }
//!     }
//! }
//!
//! type DisplayGen = Conditional<IsEnum, EnumDisplay, StructDisplay>;
//! let ts = DisplayGen::generate(&spec);
//! ```
//!
//! Notes
//! - This module does not replace builders: it orchestrates their invocation.
//! - Prefer `syn::Result<TokenStream2>` for generators that can fail — you can add a
//!   parallel `ResultCodeGen` trait mirroring this one if needed.
//!
//! When to use CodeGen (guidance)
//! - Use it when:
//!   - You naturally split emission into multiple parts (e.g., static data + helper methods).
//!   - You need to branch on IR shape (struct vs enum) with clearly separated generators.
//!   - You want small, independently testable generation units and a declarative composition line
//!     (e.g., `type Full = Chain<ModGen, ImplGen>`).
//! - Avoid it when:
//!   - A single ImplBuilder/quote! block is already clear and self-contained.
//!   - The extra indirection would obscure what gets generated.
//!
//! Philosophy
//! - CodeGen favors “dumb composition over smart inference”. It doesn’t try to be clever — it makes
//!   sequencing and branching explicit, keeps state out of the way, and leans on our existing
//!   primitives (ImplBuilder/MatchArmBuilder/static_slice_mod) to do the real work.

use proc_macro2::TokenStream as TokenStream2;

/// A generator that transforms an input into generated tokens (or any Output).
pub trait CodeGen<Input> {
    type Output;
    fn generate(input: &Input) -> Self::Output;
}

/// A fallible generator that can return errors during generation.
///
/// This enables composition of generators that need to validate, parse attributes,
/// or perform other operations that might fail.
///
/// # Example
///
/// ```ignore
/// struct ValidatingGen;
/// impl ResultCodeGen<MyInput> for ValidatingGen {
///     type Output = TokenStream;
///     fn generate(input: &MyInput) -> syn::Result<Self::Output> {
///         if !input.is_valid() {
///             return Err(syn::Error::new(Span::call_site(), "invalid input"));
///         }
///         Ok(quote! { /* ... */ })
///     }
/// }
/// ```
pub trait ResultCodeGen<Input> {
    type Output;
    fn generate(input: &Input) -> syn::Result<Self::Output>;
}

/// A boolean test on an input, used for branching combinators.
pub trait Predicate<Input> {
    fn test(input: &Input) -> bool;
}

/// A fallible predicate that can return errors during testing.
pub trait ResultPredicate<Input> {
    fn test(input: &Input) -> syn::Result<bool>;
}

/// A pure transformation between stages (for advanced pipelines).
pub trait Transform<From, To> {
    fn transform(input: &From) -> To;
}

/// Run A then B; concatenate TokenStream2 outputs.
pub struct Chain<A, B>(core::marker::PhantomData<(A, B)>);

impl<Input, A, B> CodeGen<Input> for Chain<A, B>
where
    A: CodeGen<Input, Output = TokenStream2>,
    B: CodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> Self::Output {
        let a = A::generate(input);
        let b = B::generate(input);
        quote::quote! { #a #b }
    }
}

/// If P::test(input) { T } else { F } — branch generator.
pub struct Conditional<P, T, F>(core::marker::PhantomData<(P, T, F)>);

impl<Input, P, T, F> CodeGen<Input> for Conditional<P, T, F>
where
    P: Predicate<Input>,
    T: CodeGen<Input, Output = TokenStream2>,
    F: CodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> Self::Output {
        if P::test(input) { T::generate(input) } else { F::generate(input) }
    }
}

/// Noop generator - produces empty token stream.
///
/// Useful as the "false" branch in conditionals or as an identity element.
pub struct Noop;

impl<Input> CodeGen<Input> for Noop {
    type Output = TokenStream2;
    fn generate(_: &Input) -> Self::Output {
        TokenStream2::new()
    }
}

/// Map input transformation before generation.
///
/// Applies function `F` to transform input, then generates using `G`.
/// This enables input preprocessing or type conversions.
pub struct Map<G, F>(core::marker::PhantomData<(G, F)>);

/// Function trait for input transformation.
pub trait MapFn<From> {
    type To;
    fn map(from: &From) -> Self::To;
}

impl<From, G, F> CodeGen<From> for Map<G, F>
where
    F: MapFn<From>,
    G: CodeGen<F::To, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &From) -> Self::Output {
        let mapped = F::map(input);
        G::generate(&mapped)
    }
}

// ============================================================================
// Fallible Combinators (ResultCodeGen)
// ============================================================================

/// Chain two fallible generators sequentially, concatenating their outputs.
///
/// Both generators must produce `TokenStream2` and can fail with `syn::Error`.
/// If either fails, the error is propagated.
pub struct ResultChain<A, B>(core::marker::PhantomData<(A, B)>);

impl<Input, A, B> ResultCodeGen<Input> for ResultChain<A, B>
where
    A: ResultCodeGen<Input, Output = TokenStream2>,
    B: ResultCodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        let a = A::generate(input)?;
        let b = B::generate(input)?;
        Ok(quote::quote! { #a #b })
    }
}

/// Chain a fallible generator with an infallible one.
///
/// Useful when you need validation first, then straightforward generation.
pub struct TryChain<A, B>(core::marker::PhantomData<(A, B)>);

impl<Input, A, B> ResultCodeGen<Input> for TryChain<A, B>
where
    A: ResultCodeGen<Input, Output = TokenStream2>,
    B: CodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        let a = A::generate(input)?;
        let b = B::generate(input);
        Ok(quote::quote! { #a #b })
    }
}

/// Conditional generator with fallible branches.
///
/// The predicate can fail, and both branches can fail.
pub struct ResultConditional<P, T, F>(core::marker::PhantomData<(P, T, F)>);

impl<Input, P, T, F> ResultCodeGen<Input> for ResultConditional<P, T, F>
where
    P: ResultPredicate<Input>,
    T: ResultCodeGen<Input, Output = TokenStream2>,
    F: ResultCodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        if P::test(input)? {
            T::generate(input)
        } else {
            F::generate(input)
        }
    }
}

/// Conditional with infallible predicate and fallible branches.
pub struct TryConditional<P, T, F>(core::marker::PhantomData<(P, T, F)>);

impl<Input, P, T, F> ResultCodeGen<Input> for TryConditional<P, T, F>
where
    P: Predicate<Input>,
    T: ResultCodeGen<Input, Output = TokenStream2>,
    F: ResultCodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        if P::test(input) {
            T::generate(input)
        } else {
            F::generate(input)
        }
    }
}

// ============================================================================
// Adapters (Convert between CodeGen and ResultCodeGen)
// ============================================================================

/// Lift an infallible generator into a fallible one.
///
/// This allows composing `CodeGen` with `ResultCodeGen` seamlessly.
pub struct Lift<G>(core::marker::PhantomData<G>);

impl<Input, G> ResultCodeGen<Input> for Lift<G>
where
    G: CodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> syn::Result<Self::Output> {
        Ok(G::generate(input))
    }
}

/// Wrap a fallible generator to panic on errors (use with caution).
///
/// Only use this when you're certain the generator won't fail, or when
/// you want to propagate panics instead of returning Results.
pub struct Unwrap<G>(core::marker::PhantomData<G>);

impl<Input, G> CodeGen<Input> for Unwrap<G>
where
    G: ResultCodeGen<Input, Output = TokenStream2>,
{
    type Output = TokenStream2;
    fn generate(input: &Input) -> Self::Output {
        G::generate(input).expect("ResultCodeGen failed")
    }
}

// ============================================================================
// Helper Predicates
// ============================================================================

/// Always-true predicate.
pub struct AlwaysTrue;
impl<T> Predicate<T> for AlwaysTrue {
    fn test(_: &T) -> bool {
        true
    }
}

/// Always-false predicate.
pub struct AlwaysFalse;
impl<T> Predicate<T> for AlwaysFalse {
    fn test(_: &T) -> bool {
        false
    }
}

// ============================================================================
// Macro for Sequencing
// ============================================================================

/// Sequence multiple generators into a chain.
///
/// Expands to nested `Chain` combinators, left-to-right.
///
/// # Example
///
/// ```ignore
/// type MyGen = seq![ModuleGen, TraitGen, InherentGen];
/// // Expands to: Chain<Chain<ModuleGen, TraitGen>, InherentGen>
/// ```
#[macro_export]
macro_rules! seq {
    ($single:ty) => { $single };
    ($first:ty, $second:ty) => {
        $crate::common::gen::Chain<$first, $second>
    };
    ($first:ty, $($rest:ty),+ $(,)?) => {
        $crate::common::gen::Chain<$first, $crate::seq![$($rest),+]>
    };
}

/// Sequence multiple fallible generators into a chain.
///
/// Expands to nested `ResultChain` combinators, left-to-right.
/// If any generator fails, the error is propagated.
///
/// # Example
///
/// ```ignore
/// type MyDerive = try_seq![ValidateGen, ParseGen, GenerateGen];
/// // Expands to: ResultChain<ResultChain<ValidateGen, ParseGen>, GenerateGen>
/// let result = MyDerive::generate(&input)?;
/// ```
#[macro_export]
macro_rules! try_seq {
    ($single:ty) => { $single };
    ($first:ty, $second:ty) => {
        $crate::common::gen::ResultChain<$first, $second>
    };
    ($first:ty, $($rest:ty),+ $(,)?) => {
        $crate::common::gen::ResultChain<$first, $crate::try_seq![$($rest),+]>
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    struct Hello; impl CodeGen<()> for Hello { type Output = TokenStream2; fn generate(_: &()) -> Self::Output { quote!{ fn hello(){} } } }
    struct World; impl CodeGen<()> for World { type Output = TokenStream2; fn generate(_: &()) -> Self::Output { quote!{ fn world(){} } } }
    struct Foo; impl CodeGen<()> for Foo { type Output = TokenStream2; fn generate(_: &()) -> Self::Output { quote!{ fn foo(){} } } }
    struct Yes; impl Predicate<()> for Yes { fn test(_: &()) -> bool { true } }
    struct No; impl Predicate<()> for No { fn test(_: &()) -> bool { false } }

    #[test] fn chain_works() {
        type Both = Chain<Hello, World>;
        let ts = Both::generate(&());
        let s = ts.to_string();
        assert!(s.contains("fn hello"));
        assert!(s.contains("fn world"));
    }

    #[test] fn conditional_works() {
        type Then = Conditional<Yes, Hello, World>;
        type Else = Conditional<No, Hello, World>;
        assert!(Then::generate(&()).to_string().contains("fn hello"));
        assert!(Else::generate(&()).to_string().contains("fn world"));
    }

    #[test]
    fn noop_generates_nothing() {
        let ts = Noop::generate(&());
        assert!(ts.is_empty());
    }

    #[test]
    fn conditional_with_noop() {
        type OptionalHello = Conditional<Yes, Hello, Noop>;
        type OptionalWorld = Conditional<No, World, Noop>;

        let a = OptionalHello::generate(&());
        let b = OptionalWorld::generate(&());

        assert!(a.to_string().contains("fn hello"));
        assert!(b.is_empty());
    }

    #[test]
    fn seq_macro_chains_multiple() {
        type All = seq![Hello, World, Foo];
        let ts = All::generate(&());
        let s = ts.to_string();
        assert!(s.contains("fn hello"));
        assert!(s.contains("fn world"));
        assert!(s.contains("fn foo"));
    }

    #[test]
    fn map_transforms_input() {
        struct IntToString;
        impl MapFn<i32> for IntToString {
            type To = String;
            fn map(from: &i32) -> String {
                from.to_string()
            }
        }

        struct StringGen;
        impl CodeGen<String> for StringGen {
            type Output = TokenStream2;
            fn generate(input: &String) -> Self::Output {
                let s = input.clone();
                quote! { const VALUE: &str = #s; }
            }
        }

        type MappedGen = Map<StringGen, IntToString>;
        let ts = MappedGen::generate(&42);
        let s = ts.to_string();
        assert!(s.contains("\"42\""));
    }

    // ========== ResultCodeGen Tests ==========

    struct SuccessGen;
    impl ResultCodeGen<i32> for SuccessGen {
        type Output = TokenStream2;
        fn generate(input: &i32) -> syn::Result<Self::Output> {
            Ok(quote! { const VALUE: i32 = #input; })
        }
    }

    struct FailGen;
    impl ResultCodeGen<i32> for FailGen {
        type Output = TokenStream2;
        fn generate(_: &i32) -> syn::Result<Self::Output> {
            Err(syn::Error::new(proc_macro2::Span::call_site(), "intentional failure"))
        }
    }

    struct ValidPredicate;
    impl ResultPredicate<i32> for ValidPredicate {
        fn test(input: &i32) -> syn::Result<bool> {
            Ok(*input > 0)
        }
    }

    #[test]
    fn result_chain_success() {
        type Both = ResultChain<SuccessGen, SuccessGen>;
        let result = <Both as ResultCodeGen<i32>>::generate(&42);
        assert!(result.is_ok());
        let ts = result.unwrap();
        let s = ts.to_string();
        assert_eq!(s.matches("const VALUE").count(), 2);
    }

    #[test]
    fn result_chain_failure_propagates() {
        type Mixed = ResultChain<SuccessGen, FailGen>;
        let result = <Mixed as ResultCodeGen<i32>>::generate(&42);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("intentional failure"));
    }

    #[test]
    fn try_chain_works() {
        struct InfallibleGen;
        impl CodeGen<i32> for InfallibleGen {
            type Output = TokenStream2;
            fn generate(input: &i32) -> Self::Output {
                quote! { fn infallible_value() -> i32 { #input } }
            }
        }

        type Mixed = TryChain<SuccessGen, InfallibleGen>;
        let result = <Mixed as ResultCodeGen<i32>>::generate(&42);
        assert!(result.is_ok());
        let ts = result.unwrap();
        let s = ts.to_string();
        assert!(s.contains("VALUE"));
        assert!(s.contains("infallible_value"));
    }

    #[test]
    fn result_conditional_works() {
        type Cond = ResultConditional<ValidPredicate, SuccessGen, FailGen>;

        // Positive input -> success branch
        let result = <Cond as ResultCodeGen<i32>>::generate(&42);
        assert!(result.is_ok());

        // Negative input -> fail branch -> error
        let result = <Cond as ResultCodeGen<i32>>::generate(&-1);
        assert!(result.is_err());
    }

    #[test]
    fn try_conditional_works() {
        struct PositivePred;
        impl Predicate<i32> for PositivePred {
            fn test(input: &i32) -> bool {
                *input > 0
            }
        }

        type Cond = TryConditional<PositivePred, SuccessGen, FailGen>;

        let result = <Cond as ResultCodeGen<i32>>::generate(&42);
        assert!(result.is_ok());

        let result = <Cond as ResultCodeGen<i32>>::generate(&-1);
        assert!(result.is_err());
    }

    #[test]
    fn lift_adapts_codegen_to_result() {
        type Lifted = Lift<Hello>;
        let result = <Lifted as ResultCodeGen<()>>::generate(&());
        assert!(result.is_ok());
        assert!(result.unwrap().to_string().contains("hello"));
    }

    #[test]
    #[should_panic(expected = "ResultCodeGen failed")]
    fn unwrap_panics_on_error() {
        type Unwrapped = Unwrap<FailGen>;
        let _ = Unwrapped::generate(&42); // Should panic
    }

    #[test]
    fn try_seq_macro_chains_fallible() {
        type All = try_seq![SuccessGen, SuccessGen, SuccessGen];
        let result = <All as ResultCodeGen<i32>>::generate(&42);
        assert!(result.is_ok());
        let s = result.unwrap().to_string();
        assert_eq!(s.matches("const VALUE").count(), 3);
    }
}
