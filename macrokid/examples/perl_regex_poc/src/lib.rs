use proc_macro::TokenStream;
use syn::{parse_macro_input, Expr};
use quote::quote;

/// Proof of concept: Perl-like regex matching
/// 
/// Usage: regex_match!(text =~ "pattern")
/// This is a simplified version to demonstrate the concept
#[proc_macro]
pub fn regex_match(input: TokenStream) -> TokenStream {
    let regex_expr = parse_macro_input!(input as RegexMatchExpr);
    perl_regex_impl::expand_regex_match(regex_expr).into()
}

/// Proof of concept: Perl-like substitution
/// 
/// Usage: regex_subst!(text, "pattern", "replacement")
#[proc_macro]  
pub fn regex_subst(input: TokenStream) -> TokenStream {
    let subst_expr = parse_macro_input!(input as RegexSubstExpr);
    perl_regex_impl::expand_regex_subst(subst_expr).into()
}

// Simplified parsing structures for proof of concept
struct RegexMatchExpr {
    target: Expr,
    pattern: syn::LitStr,
}

impl syn::parse::Parse for RegexMatchExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let target = input.parse()?;
        // Support both: `target, "pat"` and `target =~ "pat"` (POC)
        let lookahead = input.lookahead1();
        let pattern = if lookahead.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            input.parse()?
        } else {
            input.parse::<syn::Token![=]>()?;
            // Not all tokens like `~` are allowed in outer macro input; this branch
            // remains for documentation, but examples use the comma form.
            let _ = input.parse::<syn::Token![~]>().ok();
            input.parse()?
        };
        
        Ok(RegexMatchExpr { target, pattern })
    }
}

struct RegexSubstExpr {
    target: Expr,
    pattern: syn::LitStr,
    replacement: syn::LitStr,
}

impl syn::parse::Parse for RegexSubstExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let target = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let pattern = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let replacement = input.parse()?;
        
        Ok(RegexSubstExpr { target, pattern, replacement })
    }
}

mod perl_regex_impl {
    use super::*;
    use proc_macro2::TokenStream as TokenStream2;

    pub fn expand_regex_match(expr: RegexMatchExpr) -> TokenStream2 {
        let target = &expr.target;
        let pattern = expr.pattern.value();

        // Generate regex matching code using the framework's patterns
        quote! {
            {
                use perl_regex_runtime::Regex;
                use perl_regex_runtime::PerlRegexMatch;
                
                // Static regex for performance (compile once, use many times)
                static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                let re = REGEX.get_or_init(|| {
                    Regex::new(#pattern).expect("Invalid regex pattern")
                });

                // Perl-like match result
                if let Some(caps) = re.captures(&#target) {
                    PerlRegexMatch::new_match(&#target, caps)
                } else {
                    PerlRegexMatch::no_match(&#target)
                }
            }
        }
    }

    pub fn expand_regex_subst(expr: RegexSubstExpr) -> TokenStream2 {
        let target = &expr.target;
        let pattern = expr.pattern.value();
        let replacement = expr.replacement.value();

        quote! {
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
        }
    }
}

// Note: Runtime support would typically be in a separate non-proc-macro crate
