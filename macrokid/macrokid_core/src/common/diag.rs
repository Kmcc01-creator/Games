use proc_macro2::Span;
use quote::ToTokens;
use syn::{spanned::Spanned, Error as SynError};

/// Create a span-aware error on the given AST node.
pub fn err_on<T: Spanned + ToTokens>(node: &T, msg: &str) -> SynError {
    SynError::new_spanned(node, msg)
}

/// Create a span-aware error with an extra note appended to the message.
pub fn suggest_with_note<T: Spanned + ToTokens>(node: &T, msg: &str, note: &str) -> SynError {
    let combined = format!("{} (note: {})", msg, note);
    SynError::new_spanned(node, combined)
}

/// Create an error at a specific span
pub fn err_at_span(span: Span, msg: &str) -> SynError {
    SynError::new(span, msg)
}

/// Collector that aggregates multiple syn::Error values and returns a single error.
#[derive(Default)]
pub struct Collector {
    agg: Option<SynError>,
}

impl Collector {
    pub fn new() -> Self { Self { agg: None } }
    pub fn push(&mut self, err: SynError) {
        if let Some(ref mut a) = self.agg {
            a.combine(err);
        } else {
            self.agg = Some(err);
        }
    }
    pub fn is_empty(&self) -> bool { self.agg.is_none() }
    pub fn has_errors(&self) -> bool { self.agg.is_some() }
    pub fn into_result<T>(self, ok: T) -> Result<T, SynError> { self.agg.map_or(Ok(ok), Err) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_err_on_contains_message() {
        let ident: syn::Ident = parse_quote!(MyType);
        let err = err_on(&ident, "oops");
        assert!(format!("{}", err).contains("oops"));
    }

    #[test]
    fn test_suggest_with_note_contains_note() {
        let ident: syn::Ident = parse_quote!(Field);
        let err = suggest_with_note(&ident, "bad", "try something else");
        let msg = format!("{}", err);
        assert!(msg.contains("bad"));
        assert!(msg.contains("try something else"));
    }
}
