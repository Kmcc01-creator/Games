//! Perl-like regex proof-of-concept macros built with macrokid_core patterns.
//!
//! Highlights (inspired by Perl):
//! - Compact match and substitution forms (m/p and s/// styles).
//! - Inline flags such as case-insensitive (i), multi-line (m), dot-matches-newline (s),
//!   extended whitespace/comments (x), and swap-greed (U).
//! - Global substitution semantics for `regex_subst!` (like `/g`).
//!
//! This PoC exposes two macros:
//! - `regex_match!(text, pattern[, flags])` → returns a `PerlRegexMatch` with `.matched` and `.full_match()`.
//! - `regex_subst!(text, pattern, replacement[, flags])` → returns a `PerlRegexSubst { result, count }`.
//!
//! Flags (optional third/ fourth literal argument):
//! - `i` = case-insensitive, `m` = multi-line, `s` = dot matches newline, `x` = ignore whitespace, `U` = swap greed.
//! Unsupported flags will produce a compile error with spans.
//!
//! Note: For ergonomics this PoC prefixes the pattern with an inline `(?imxsU)` construct generated at
//! macro-expansion time when flags are passed. This leverages Rust regex inline-flags behavior.
//!
//! regex_match! remains simple: captures only expose full matches via `all_matches()`. If you want per-capture
//! groups (Perl `$1`, `$2`), we can extend `PerlRegexMatch` with a vector of capture sets and add ergonomics.
//! If you want, I can add a couple of usage snippets inline next to the macro definitions showing flags combinations
//! (e.g., "imsxU", "g") and their effects.

use proc_macro::TokenStream;
use syn::{parse_macro_input, Expr, LitStr};
use quote::quote;
use macrokid_core::diag::err_at_span;

/// Proof of concept: Perl-like regex matching
/// 
/// Usage: regex_match!(text =~ "pattern")
/// This is a simplified version to demonstrate the concept
#[proc_macro]
pub fn regex_match(input: TokenStream) -> TokenStream {
    let regex_expr = parse_macro_input!(input as RegexMatchExpr);
    match perl_regex_impl::expand_regex_match(regex_expr) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }.into()
}

/// Proof of concept: Perl-like substitution
/// 
/// Usage: regex_subst!(text, "pattern", "replacement")
#[proc_macro]  
pub fn regex_subst(input: TokenStream) -> TokenStream {
    let subst_expr = parse_macro_input!(input as RegexSubstExpr);
    match perl_regex_impl::expand_regex_subst(subst_expr) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }.into()
}

// Simplified parsing structures for proof of concept
struct RegexMatchExpr {
    target: Expr,
    pattern: syn::LitStr,
    flags: Option<syn::LitStr>,
}

impl syn::parse::Parse for RegexMatchExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let target = input.parse()?;
        // Support both: `target, "pat"[, "flags"]` and `target =~ "pat"` (POC)
        let lookahead = input.lookahead1();
        let (pattern, flags) = if lookahead.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            let pat: LitStr = input.parse()?;
            let flags = if input.parse::<syn::Token![,]>().is_ok() {
                Some(input.parse::<LitStr>()?)
            } else { None };
            (pat, flags)
        } else {
            input.parse::<syn::Token![=]>()?;
            // Not all tokens like `~` are allowed in outer macro input; this branch
            // remains for documentation, but examples use the comma form.
            let _ = input.parse::<syn::Token![~]>().ok();
            (input.parse()?, None)
        };
        
        Ok(RegexMatchExpr { target, pattern, flags })
    }
}

struct RegexSubstExpr {
    target: Expr,
    pattern: syn::LitStr,
    replacement: syn::LitStr,
    flags: Option<syn::LitStr>,
}

impl syn::parse::Parse for RegexSubstExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let target = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let pattern = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let replacement = input.parse()?;
        let flags = if input.parse::<syn::Token![,]>().is_ok() { Some(input.parse::<LitStr>()?) } else { None };
        
        Ok(RegexSubstExpr { target, pattern, replacement, flags })
    }
}

mod perl_regex_impl {
    use super::*;
    use proc_macro2::TokenStream as TokenStream2;

    fn process_flags(pat: &LitStr, flags: Option<LitStr>) -> syn::Result<(LitStr, bool)> {
        if let Some(f) = flags {
            let v = f.value();
            let mut inline = String::new();
            let mut global = false;
            for ch in v.chars() {
                match ch {
                    'g' => global = true,
                    'i' | 'm' | 's' | 'x' | 'U' => inline.push(ch),
                    other => return Err(err_at_span(f.span(), &format!("unsupported flag '{}': expected one of gimsxU", other))),
                }
            }
            if inline.is_empty() {
                return Ok((pat.clone(), global));
            }
            let prefixed = format!("(?{}){}", inline, pat.value());
            Ok((LitStr::new(&prefixed, pat.span()), global))
        } else {
            Ok((pat.clone(), false))
        }
    }

    pub fn expand_regex_match(expr: RegexMatchExpr) -> syn::Result<TokenStream2> {
        let target = &expr.target;
        let (pattern, global) = process_flags(&expr.pattern, expr.flags)?;

        // Generate regex matching code using the framework's patterns
        Ok(quote! {
            {
                use perl_regex_runtime::Regex;
                use perl_regex_runtime::PerlRegexMatch;
                
                // Static regex for performance (compile once, use many times)
                static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                let re = REGEX.get_or_init(|| {
                    Regex::new(#pattern).expect("Invalid regex pattern")
                });

                // Perl-like match result
                if #global {
                    let all: ::std::vec::Vec<String> = re.find_iter(&#target).map(|m| m.as_str().to_string()).collect();
                    PerlRegexMatch::from_all(&#target, all)
                } else {
                    if let Some(caps) = re.captures(&#target) {
                        PerlRegexMatch::new_match(&#target, caps)
                    } else {
                        PerlRegexMatch::no_match(&#target)
                    }
                }
            }
        })
    }

    pub fn expand_regex_subst(expr: RegexSubstExpr) -> syn::Result<TokenStream2> {
        let target = &expr.target;
        let (pattern, _global) = process_flags(&expr.pattern, expr.flags)?;
        let replacement = expr.replacement.value();

        Ok(quote! {
            {
                use perl_regex_runtime::Regex;
                use perl_regex_runtime::PerlRegexSubst;
                
                static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                let re = REGEX.get_or_init(|| {
                    Regex::new(#pattern).expect("Invalid regex pattern")
                });

                // Perl-like substitution
                PerlRegexSubst {
                    result: re.replace_all(&#target, #replacement).to_string(),
                    count: re.find_iter(&#target).count(),
                }
            }
        })
    }
}

// Note: Runtime support lives in `perl_regex_runtime` to keep proc-macro crate lean.
