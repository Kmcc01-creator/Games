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

/// A boolean test on an input, used for branching combinators.
pub trait Predicate<Input> {
    fn test(input: &Input) -> bool;
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    struct Hello; impl CodeGen<()> for Hello { type Output = TokenStream2; fn generate(_: &()) -> Self::Output { quote!{ fn hello(){} } } }
    struct World; impl CodeGen<()> for World { type Output = TokenStream2; fn generate(_: &()) -> Self::Output { quote!{ fn world(){} } } }
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
}
