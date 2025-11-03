//! Minimal attribute parser POC for macrokid
//!
//! This is a proof-of-concept fast parser for macrokid's specific attribute patterns.
//! It's optimized for the case where we know exactly what we're looking for.

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, TokenStream, TokenTree};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub struct UniformAttr {
    pub set: u32,
    pub binding: u32,
    pub stages: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceAttr {
    Uniform(UniformAttr),
    Texture { set: u32, binding: u32, stages: Option<String> },
    Sampler { set: u32, binding: u32, stages: Option<String> },
    Combined { set: u32, binding: u32, stages: Option<String> },
}

/// Fast parser for macrokid resource attributes
///
/// Parses attributes like:
/// - `#[uniform(set = 0, binding = 1, stages = "vs|fs")]`
/// - `#[texture(set = 0, binding = 1)]`
/// etc.
///
/// This is much faster than syn because:
/// 1. We know exactly what tokens to expect
/// 2. No need to build a full AST
/// 3. Direct pattern matching on token trees
/// 4. Fail-fast on unexpected patterns
pub fn parse_resource_attr(tokens: TokenStream) -> Result<ResourceAttr, String> {
    let mut iter = tokens.into_iter().peekable();

    // Skip leading '#'
    match iter.next() {
        Some(TokenTree::Punct(p)) if p.as_char() == '#' => {}
        _ => return Err("Expected '#'".to_string()),
    }

    // Parse the attribute group [...]
    let group = match iter.next() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket => g,
        _ => return Err("Expected '[...]'".to_string()),
    };

    parse_attr_content(group.stream())
}

fn parse_attr_content(tokens: TokenStream) -> Result<ResourceAttr, String> {
    let mut iter = tokens.into_iter().peekable();

    // Get attribute name (uniform, texture, sampler, combined)
    let attr_name = match iter.next() {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
        _ => return Err("Expected attribute name".to_string()),
    };

    // Parse the parameter group (...)
    let params = match iter.next() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => g,
        _ => return Err("Expected '(...)'".to_string()),
    };

    let (set, binding, stages) = parse_params(params.stream())?;

    match attr_name.as_str() {
        "uniform" => Ok(ResourceAttr::Uniform(UniformAttr { set, binding, stages })),
        "texture" => Ok(ResourceAttr::Texture { set, binding, stages }),
        "sampler" => Ok(ResourceAttr::Sampler { set, binding, stages }),
        "combined" => Ok(ResourceAttr::Combined { set, binding, stages }),
        _ => Err(format!("Unknown attribute: {}", attr_name)),
    }
}

fn parse_params(tokens: TokenStream) -> Result<(u32, u32, Option<String>), String> {
    let mut iter = tokens.into_iter().peekable();
    let mut set = None;
    let mut binding = None;
    let mut stages = None;

    loop {
        // Parse key
        let key = match iter.next() {
            Some(TokenTree::Ident(ident)) => ident.to_string(),
            None => break,
            _ => return Err("Expected parameter name".to_string()),
        };

        // Expect '='
        match iter.next() {
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => {}
            _ => return Err("Expected '='".to_string()),
        }

        // Parse value based on key
        match key.as_str() {
            "set" => {
                set = Some(parse_int(&mut iter)?);
            }
            "binding" => {
                binding = Some(parse_int(&mut iter)?);
            }
            "stages" => {
                stages = Some(parse_string(&mut iter)?);
            }
            _ => return Err(format!("Unknown parameter: {}", key)),
        }

        // Check for comma or end
        match iter.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == ',' => {
                iter.next(); // consume comma
            }
            None => break,
            _ => {}
        }
    }

    let set = set.ok_or("Missing 'set' parameter")?;
    let binding = binding.ok_or("Missing 'binding' parameter")?;

    Ok((set, binding, stages))
}

fn parse_int<I>(iter: &mut Peekable<I>) -> Result<u32, String>
where
    I: Iterator<Item = TokenTree>,
{
    match iter.next() {
        Some(TokenTree::Literal(lit)) => {
            let s = lit.to_string();
            s.parse().map_err(|_| format!("Invalid integer: {}", s))
        }
        _ => Err("Expected integer literal".to_string()),
    }
}

fn parse_string<I>(iter: &mut Peekable<I>) -> Result<String, String>
where
    I: Iterator<Item = TokenTree>,
{
    match iter.next() {
        Some(TokenTree::Literal(lit)) => {
            let s = lit.to_string();
            // Remove quotes
            if s.starts_with('"') && s.ends_with('"') {
                Ok(s[1..s.len() - 1].to_string())
            } else {
                Err(format!("Expected string literal, got: {}", s))
            }
        }
        _ => Err("Expected string literal".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_parse_uniform() {
        let tokens = quote! {
            #[uniform(set = 0, binding = 1, stages = "vs|fs")]
        };

        let result = parse_resource_attr(tokens).unwrap();
        match result {
            ResourceAttr::Uniform(u) => {
                assert_eq!(u.set, 0);
                assert_eq!(u.binding, 1);
                assert_eq!(u.stages, Some("vs|fs".to_string()));
            }
            _ => panic!("Expected Uniform variant"),
        }
    }

    #[test]
    fn test_parse_texture() {
        let tokens = quote! {
            #[texture(set = 0, binding = 2)]
        };

        let result = parse_resource_attr(tokens).unwrap();
        match result {
            ResourceAttr::Texture { set, binding, stages } => {
                assert_eq!(set, 0);
                assert_eq!(binding, 2);
                assert_eq!(stages, None);
            }
            _ => panic!("Expected Texture variant"),
        }
    }
}
