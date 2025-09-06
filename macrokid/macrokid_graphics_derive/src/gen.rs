//! Prototype: Composable, zero-cost code generation combinators for derive crates.
//!
//! This prototype lives in macrokid_graphics_derive to refine ergonomics before promoting
//! a minimal version into macrokid_core (behind a feature flag) if it proves broadly useful.
//!
//! Traits
//! - `CodeGen<Input>`: generate Output (typically `TokenStream2`).
//! - `Predicate<Input>`: branch generation based on input properties.
//! - `Transform<From, To>`: pure input transformation for pipelines.
//!
//! Combinators (zero-cost, PhantomData-based)
//! - `Chain<A, B>`: run A then B, concatenating TokenStream2.
//! - `Conditional<P, T, F>`: pick T or F based on predicate P.
//!
//! Example
//! ```ignore
//! use crate::gen::*;
//! use quote::quote;
//!
//! struct HelloGen;
//! impl CodeGen<()> for HelloGen {
//!     type Output = proc_macro2::TokenStream;
//!     fn generate(_: &()) -> Self::Output { quote! { fn hello() {} } }
//! }
//!
//! struct WorldGen;
//! impl CodeGen<()> for WorldGen {
//!     type Output = proc_macro2::TokenStream;
//!     fn generate(_: &()) -> Self::Output { quote! { fn world() {} } }
//! }
//!
//! type Both = Chain<HelloGen, WorldGen>;
//! let ts = Both::generate(&());
//! // ts now contains two fn items: hello and world
//! ```

use proc_macro2::TokenStream as TokenStream2;

pub trait CodeGen<Input> {
    type Output;
    fn generate(input: &Input) -> Self::Output;
}

pub trait Predicate<Input> {
    fn test(input: &Input) -> bool;
}

pub trait Transform<From, To> {
    fn transform(input: &From) -> To;
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    struct HelloGen;
    impl CodeGen<()> for HelloGen {
        type Output = TokenStream2;
        fn generate(_: &()) -> Self::Output { quote! { fn hello() {} } }
    }

    struct WorldGen;
    impl CodeGen<()> for WorldGen {
        type Output = TokenStream2;
        fn generate(_: &()) -> Self::Output { quote! { fn world() {} } }
    }

    struct AlwaysTrue;
    impl Predicate<()> for AlwaysTrue { fn test(_: &()) -> bool { true } }
    struct AlwaysFalse;
    impl Predicate<()> for AlwaysFalse { fn test(_: &()) -> bool { false } }

    #[test]
    fn chain_concatenates() {
        type Both = Chain<HelloGen, WorldGen>;
        let ts = Both::generate(&());
        let s = ts.to_string();
        assert!(s.contains("fn hello"));
        assert!(s.contains("fn world"));
    }

    #[test]
    fn conditional_branches() {
        type Then = Conditional<AlwaysTrue, HelloGen, WorldGen>;
        type Else = Conditional<AlwaysFalse, HelloGen, WorldGen>;
        let a = Then::generate(&());
        let b = Else::generate(&());
        assert!(a.to_string().contains("fn hello"));
        assert!(b.to_string().contains("fn world"));
    }
}

