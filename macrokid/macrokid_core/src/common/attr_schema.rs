use std::collections::HashMap;
use syn::Attribute;

use crate::common::attrs::{validate_attrs, AttrSpec as LowSpec, AttrType, AttrValue};
use crate::ir::{FieldSpec, TypeSpec, VariantSpec};

/// Typed wrapper around `validate_attrs` with a fluent builder API.
#[derive(Clone, Debug)]
pub struct AttrSchema {
    pub name: &'static str,
    pub specs: Vec<LowSpec>,
}

impl AttrSchema {
    pub fn new(name: &'static str) -> Self { Self { name, specs: Vec::new() } }

    pub fn req_str(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: true, ty: AttrType::Str }); self }
    pub fn req_bool(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: true, ty: AttrType::Bool }); self }
    pub fn req_int(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: true, ty: AttrType::Int }); self }
    pub fn req_float(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: true, ty: AttrType::Float }); self }

    pub fn opt_str(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: false, ty: AttrType::Str }); self }
    pub fn opt_bool(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: false, ty: AttrType::Bool }); self }
    pub fn opt_int(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: false, ty: AttrType::Int }); self }
    pub fn opt_float(mut self, key: &'static str) -> Self { self.specs.push(LowSpec { key, required: false, ty: AttrType::Float }); self }

    pub fn parse(&self, attrs: &[Attribute]) -> syn::Result<ParsedAttrs> {
        let map = validate_attrs(attrs, self.name, &self.specs)?;
        Ok(ParsedAttrs { map })
    }
}

/// Result of parsing an attribute with a schema.
#[derive(Clone, Debug)]
pub struct ParsedAttrs {
    pub map: HashMap<String, AttrValue>,
}

impl ParsedAttrs {
    pub fn get_str(&self, k: &str) -> Option<&str> {
        if let Some(AttrValue::Str(s)) = self.map.get(k) {
            Some(s.as_str())
        } else {
            None
        }
    }
    pub fn get_bool(&self, k: &str) -> Option<bool> { match self.map.get(k) { Some(AttrValue::Bool(b)) => Some(*b), _ => None } }
    pub fn get_int(&self, k: &str) -> Option<i64> { match self.map.get(k) { Some(AttrValue::Int(i)) => Some(*i), _ => None } }
    pub fn get_float(&self, k: &str) -> Option<f64> { match self.map.get(k) { Some(AttrValue::Float(f)) => Some(*f), _ => None } }

    pub fn try_get_str(&self, k: &str) -> syn::Result<&str> {
        match self.map.get(k) {
            Some(AttrValue::Str(s)) => Ok(s.as_str()),
            Some(_) => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("key '{}' is not a string", k))),
            None => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("missing required key '{}'", k))),
        }
    }
    pub fn try_get_bool(&self, k: &str) -> syn::Result<bool> {
        match self.map.get(k) {
            Some(AttrValue::Bool(b)) => Ok(*b),
            Some(_) => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("key '{}' is not a bool", k))),
            None => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("missing required key '{}'", k))),
        }
    }
    pub fn try_get_int(&self, k: &str) -> syn::Result<i64> {
        match self.map.get(k) {
            Some(AttrValue::Int(i)) => Ok(*i),
            Some(_) => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("key '{}' is not an int", k))),
            None => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("missing required key '{}'", k))),
        }
    }
    pub fn try_get_float(&self, k: &str) -> syn::Result<f64> {
        match self.map.get(k) {
            Some(AttrValue::Float(f)) => Ok(*f),
            Some(_) => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("key '{}' is not a float", k))),
            None => Err(syn::Error::new(proc_macro2::Span::call_site(), format!("missing required key '{}'", k))),
        }
    }
}

/// A set of mutually exclusive attribute schemas. At most one may be present.
#[derive(Clone, Debug, Default)]
pub struct AttrSchemaSet { entries: Vec<AttrSchema> }

impl AttrSchemaSet {
    pub fn new() -> Self { Self { entries: Vec::new() } }
    pub fn push(mut self, schema: AttrSchema) -> Self { self.entries.push(schema); self }

    /// Parse against the set. Returns None if none of the attributes are present.
    /// Errors if more than one is present or if present but invalid.
    pub fn parse(&self, attrs: &[Attribute]) -> syn::Result<Option<(String, ParsedAttrs)>> {
        let mut found: Option<(String, ParsedAttrs)> = None;
        for sch in &self.entries {
            // Try parse; if missing and schema has required keys, validate_attrs would error.
            // We want presence-only detection first.
            if attrs.iter().any(|a| a.path().is_ident(sch.name)) {
                let parsed = sch.parse(attrs)?;
                if found.is_some() {
                    return Err(syn::Error::new(proc_macro2::Span::call_site(), format!(
                        "multiple mutually exclusive attributes present: '{}' and '{}'",
                        found.as_ref().unwrap().0, sch.name
                    )));
                }
                found = Some((sch.name.to_string(), parsed));
            }
        }
        Ok(found)
    }

    /// Require exactly one attribute from the set.
    pub fn parse_exactly_one(&self, attrs: &[Attribute]) -> syn::Result<(String, ParsedAttrs)> {
        match self.parse(attrs)? {
            Some(v) => Ok(v),
            None => Err(syn::Error::new(proc_macro2::Span::call_site(), "expected one of the mutually exclusive attributes to be present")),
        }
    }
}

/// Convenience helpers for reading schemas at different AST levels
pub mod scope {
    use super::*;

    pub fn on_type(spec: &TypeSpec, schema: &AttrSchema) -> syn::Result<ParsedAttrs> {
        schema.parse(&spec.attrs)
    }
    pub fn on_variant(variant: &VariantSpec, schema: &AttrSchema) -> syn::Result<ParsedAttrs> {
        schema.parse(&variant.attrs)
    }
    pub fn on_field(field: &FieldSpec, schema: &AttrSchema) -> syn::Result<ParsedAttrs> {
        schema.parse(&field.attrs)
    }
}

/// Macro sugar to build an AttrSchemaSet with required keys per attribute.
/// Syntax:
/// exclusive_schemas![
///     uniform(set: int, binding: int),
///     texture(set: int, binding: int),
///     sampler(set: int, binding: int),
/// ]
#[macro_export]
macro_rules! exclusive_schemas {
    ( $( $name:ident ( $( $k:ident : $ty:ident ),* $(,)? ) ),+ $(,)? ) => {{
        let mut __set = $crate::common::attr_schema::AttrSchemaSet::new();
        $(
            let mut __schema = $crate::common::attr_schema::AttrSchema::new(stringify!($name));
            $(
                __schema = exclusive_schemas!(@push __schema, $k, $ty);
            )*
            __set = __set.push(__schema);
        )+
        __set
    }};
    (@push $schema:ident, $k:ident, int) => { $schema.req_int(stringify!($k)) };
    (@push $schema:ident, $k:ident, str) => { $schema.req_str(stringify!($k)) };
    (@push $schema:ident, $k:ident, bool) => { $schema.req_bool(stringify!($k)) };
    (@push $schema:ident, $k:ident, float) => { $schema.req_float(stringify!($k)) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn parse_simple_schema() {
        let schema = AttrSchema::new("cfgx")
            .req_str("name")
            .opt_bool("enabled")
            .opt_int("count");
        let attr: Attribute = parse_quote!(#[cfgx(name = "X", enabled = true, count = 2)]);
        let res = schema.parse(&[attr]).expect("ok");
        assert_eq!(res.get_str("name"), Some("X"));
        assert_eq!(res.get_bool("enabled"), Some(true));
        assert_eq!(res.get_int("count"), Some(2));
    }

    #[test]
    fn schema_set_exclusive() {
        let s1 = AttrSchema::new("uniform").req_int("binding");
        let s2 = AttrSchema::new("texture").req_int("binding");
        let set = AttrSchemaSet::new().push(s1).push(s2);
        let a: Attribute = syn::parse_quote!(#[texture(binding = 1)]);
        let out = set.parse(&[a]).unwrap();
        assert!(matches!(out, Some((ref n, _)) if n == "texture"));
    }

    #[test]
    fn parse_float_schema() {
        let schema = AttrSchema::new("primitive")
            .opt_float("size")
            .opt_float("radius")
            .req_float("scale");

        // Test with all float values present
        let attr: Attribute = parse_quote!(#[primitive(size = 1.5, radius = 2.0, scale = 3.5)]);
        let res = schema.parse(&[attr]).expect("parse should succeed");
        assert_eq!(res.get_float("size"), Some(1.5));
        assert_eq!(res.get_float("radius"), Some(2.0));
        assert_eq!(res.get_float("scale"), Some(3.5));
        assert_eq!(res.try_get_float("scale").unwrap(), 3.5);

        // Test with optional float missing
        let attr2: Attribute = parse_quote!(#[primitive(scale = 1.0)]);
        let res2 = schema.parse(&[attr2]).expect("parse should succeed");
        assert_eq!(res2.get_float("size"), None);
        assert_eq!(res2.get_float("radius"), None);
        assert_eq!(res2.get_float("scale"), Some(1.0));

        // Test with different float notations
        let attr3: Attribute = parse_quote!(#[primitive(size = 0.5, scale = 10.25)]);
        let res3 = schema.parse(&[attr3]).expect("parse should succeed");
        assert_eq!(res3.get_float("size"), Some(0.5));
        assert_eq!(res3.get_float("scale"), Some(10.25));
    }

    #[test]
    fn parse_float_required_missing() {
        let schema = AttrSchema::new("primitive").req_float("scale");

        // Missing required float should error
        let attr: Attribute = parse_quote!(#[primitive(size = 1.5)]);
        let res = schema.parse(&[attr]);
        assert!(res.is_err());
    }
}
