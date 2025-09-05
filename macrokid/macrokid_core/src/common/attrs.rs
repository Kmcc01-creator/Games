use std::collections::HashMap;
use crate::common::diag::err_at_span;
use syn::{spanned::Spanned, Attribute, Lit, Meta};

/// Extract a string value from an attribute like `#[attr_name = "value"]` or `#[attr_name("value")]`
pub fn attr_string_value(attrs: &[Attribute], attr_name: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            match &attr.meta {
                // Handle #[attr_name = "value"]
                Meta::NameValue(nv) => {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value());
                        }
                    }
                }
                // Handle #[attr_name("value")]
                Meta::List(_) => {
                    if let Ok(lit_str) = attr.parse_args::<syn::LitStr>() {
                        return Some(lit_str.value());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Extract multiple string values from an attribute like `#[attr_name("val1", "val2")]`
pub fn attr_string_list(attrs: &[Attribute], attr_name: &str) -> Option<Vec<String>> {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            let mut values = Vec::new();
            let result = attr.parse_nested_meta(|meta| {
                // Support patterns like #[attr("a", "b")] where nested items are bare strings
                if meta.path.get_ident().is_none() {
                    let lit: syn::LitStr = meta.input.parse()?;
                    values.push(lit.value());
                    return Ok(());
                }

                // Also permit #[attr(name = "value")] as an alternative form
                if let Some(_ident) = meta.path.get_ident() {
                    let lit: syn::LitStr = meta.value()?.parse()?;
                    values.push(lit.value());
                    return Ok(());
                }

                Ok(())
            });

            if result.is_ok() && !values.is_empty() {
                return Some(values);
            }
        }
    }
    None
}

/// Check if an attribute with the given name exists
pub fn has_attr(attrs: &[Attribute], attr_name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(attr_name))
}

/// Check for a marker/flag attribute with no arguments: `#[flag]`
pub fn has_flag(attrs: &[Attribute], attr_name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(attr_name) && matches!(&attr.meta, Meta::Path(_)))
}

/// Extract a boolean value from attributes like `#[attr(true)]` or `#[attr = true]`.
pub fn attr_bool_value(attrs: &[Attribute], attr_name: &str) -> Option<bool> {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            match &attr.meta {
                Meta::NameValue(nv) => {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Bool(lit_bool) = &expr_lit.lit {
                            return Some(lit_bool.value);
                        }
                    }
                }
                Meta::List(_) => {
                    if let Ok(lit_bool) = attr.parse_args::<syn::LitBool>() {
                        return Some(lit_bool.value());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Extract an integer value from attributes like `#[attr(123)]` or `#[attr = 123]`.
pub fn attr_int_value(attrs: &[Attribute], attr_name: &str) -> Option<i64> {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            match &attr.meta {
                Meta::NameValue(nv) => {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Int(lit_int) = &expr_lit.lit {
                            if let Ok(v) = lit_int.base10_parse::<i64>() {
                                return Some(v);
                            }
                        }
                    }
                }
                Meta::List(_) => {
                    if let Ok(lit_int) = attr.parse_args::<syn::LitInt>() {
                        if let Ok(v) = lit_int.base10_parse::<i64>() {
                            return Some(v);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Parse complex nested attributes like #[display(name = "CustomName", format = "fmt")]
pub fn parse_nested_attrs(attrs: &[Attribute], attr_name: &str) -> syn::Result<Vec<(String, String)>> {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            let mut pairs = Vec::new();
            let result = attr.parse_nested_meta(|meta| {
                // Get the key name
                if let Some(ident) = meta.path.get_ident() {
                    // Parse the value
                    let value: syn::LitStr = meta.value()?.parse()?;
                    pairs.push((ident.to_string(), value.value()));
                } else {
                    return Err(meta.error("expected identifier"));
                }
                Ok(())
            });
            
            if result.is_ok() {
                return Ok(pairs);
            }
        }
    }
    Ok(Vec::new())
}

/// Get a specific key from nested attributes
pub fn get_nested_attr_value(attrs: &[Attribute], attr_name: &str, key: &str) -> Option<String> {
    if let Ok(nested) = parse_nested_attrs(attrs, attr_name) {
        for (k, v) in nested {
            if k == key {
                return Some(v);
            }
        }
    }
    None
}

/// Types accepted by `validate_attrs` for keys
#[derive(Debug, Clone, Copy)]
pub enum AttrType { Str, Bool, Int }

/// Schema specification for a nested attribute key
#[derive(Debug, Clone, Copy)]
pub struct AttrSpec {
    pub key: &'static str,
    pub required: bool,
    pub ty: AttrType,
}

/// Parsed attribute value used by `validate_attrs`
#[derive(Debug, Clone)]
pub enum AttrValue {
    Str(String),
    Bool(bool),
    Int(i64),
}

/// Validate nested attributes like `#[name(key = "value", flag = true, count = 2)]` against a schema.
/// Returns a map of parsed values on success. Errors include precise spans.
pub fn validate_attrs(
    attrs: &[Attribute],
    attr_name: &str,
    schema: &[AttrSpec],
) -> syn::Result<HashMap<String, AttrValue>> {
    // Build a lookup for schema keys
    let mut spec_by_key: HashMap<&str, &AttrSpec> = HashMap::new();
    for spec in schema {
        spec_by_key.insert(spec.key, spec);
    }

    // Find the attribute
    let attr = match attrs.iter().find(|a| a.path().is_ident(attr_name)) {
        Some(a) => a,
        None => {
            // If attribute not present, only succeed if no required keys
            if schema.iter().any(|s| s.required) {
                return Err(err_at_span(proc_macro2::Span::call_site(), &format!(
                    "missing #[{}(..)] attribute with required keys",
                    attr_name
                )));
            } else {
                return Ok(HashMap::new());
            }
        }
    };

    let mut out: HashMap<String, AttrValue> = HashMap::new();

    attr.parse_nested_meta(|meta| {
        let ident = meta.path.get_ident().ok_or_else(|| meta.error("expected identifier"))?;
        let key = ident.to_string();
        let spec = spec_by_key.get(key.as_str()).ok_or_else(|| meta.error("unknown key"))?;

        if out.contains_key(&key) {
            return Err(meta.error("duplicate key"));
        }

        let val = match spec.ty {
            AttrType::Str => {
                let v: syn::LitStr = meta.value()?.parse()?;
                AttrValue::Str(v.value())
            }
            AttrType::Bool => {
                let v: syn::LitBool = meta.value()?.parse()?;
                AttrValue::Bool(v.value())
            }
            AttrType::Int => {
                let v: syn::LitInt = meta.value()?.parse()?;
                AttrValue::Int(v.base10_parse::<i64>()
                    .map_err(|_| syn::Error::new(v.span(), "expected integer"))?)
            }
        };

        out.insert(key, val);
        Ok(())
    })?;

    // Post-check required keys
    for spec in schema {
        if spec.required && !out.contains_key(spec.key) {
            return Err(syn::Error::new(attr.span(), format!("missing required key: {}", spec.key)));
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_has_flag_and_has_attr() {
        let a: Attribute = parse_quote!(#[flag]);
        let b: Attribute = parse_quote!(#[flag("x")]);
        let attrs = vec![a.clone(), b.clone()];
        assert!(has_attr(&attrs, "flag"));
        assert!(has_flag(&[a], "flag"));
        assert!(!has_flag(&[b], "flag"));
    }

    #[test]
    fn test_attr_bool_value() {
        let a: Attribute = parse_quote!(#[enabled(true)]);
        let b: Attribute = parse_quote!(#[enabled = true]);
        let c: Attribute = parse_quote!(#[enabled("no")]);
        assert_eq!(attr_bool_value(&[a], "enabled"), Some(true));
        assert_eq!(attr_bool_value(&[b], "enabled"), Some(true));
        assert_eq!(attr_bool_value(&[c], "enabled"), None);
    }

    #[test]
    fn test_attr_int_value() {
        let a: Attribute = parse_quote!(#[count(42)]);
        let b: Attribute = parse_quote!(#[count = 7]);
        let c: Attribute = parse_quote!(#[count("no")]);
        assert_eq!(attr_int_value(&[a], "count"), Some(42));
        assert_eq!(attr_int_value(&[b], "count"), Some(7));
        assert_eq!(attr_int_value(&[c], "count"), None);
    }

    #[test]
    fn test_validate_attrs_ok() {
        let attr: Attribute = parse_quote!(#[cfgx(name = "X", enabled = true, count = 2)]);
        let schema = [
            AttrSpec { key: "name", required: true, ty: AttrType::Str },
            AttrSpec { key: "enabled", required: false, ty: AttrType::Bool },
            AttrSpec { key: "count", required: false, ty: AttrType::Int },
        ];
        let map = validate_attrs(&[attr], "cfgx", &schema).expect("valid attrs");
        assert!(matches!(map.get("name"), Some(AttrValue::Str(s)) if s == "X"));
        assert!(matches!(map.get("enabled"), Some(AttrValue::Bool(true))));
        assert!(matches!(map.get("count"), Some(AttrValue::Int(2))));
    }

    #[test]
    fn test_validate_attrs_missing_required() {
        let attr: Attribute = parse_quote!(#[cfgx()]);
        let schema = [AttrSpec { key: "name", required: true, ty: AttrType::Str }];
        let err = validate_attrs(&[attr], "cfgx", &schema).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("missing required key"));
    }
}
