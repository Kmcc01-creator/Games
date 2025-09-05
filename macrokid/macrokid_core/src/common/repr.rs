use syn::{Attribute, LitInt, token::Paren};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReprKind {
    Rust,
    C,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntRepr {
    U8,
    U16,
    U32,
    U64,
    Usize,
    I8,
    I16,
    I32,
    I64,
    Isize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReprInfo {
    pub kind: Option<ReprKind>,
    pub int: Option<IntRepr>,
    pub align: Option<u64>,
    pub packed: Option<u64>, // None = not packed; Some(1) == packed; Some(n) == packed(n)
}

fn parse_align(lit: LitInt) -> syn::Result<u64> {
    Ok(lit.base10_parse::<u64>()?)
}

/// Parse a #[repr(...)] attribute into a normalized ReprInfo
pub fn parse_repr(attrs: &[Attribute]) -> syn::Result<Option<ReprInfo>> {
    let mut out = ReprInfo::default();
    let mut found = false;

    for attr in attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        found = true;
        attr.parse_nested_meta(|meta| {
            if let Some(ident) = meta.path.get_ident() {
                match ident.to_string().as_str() {
                    // Repr kinds
                    "rust" => out.kind = Some(ReprKind::Rust),
                    "C" => out.kind = Some(ReprKind::C),
                    "transparent" => out.kind = Some(ReprKind::Transparent),

                    // Integer reprs
                    "u8" => out.int = Some(IntRepr::U8),
                    "u16" => out.int = Some(IntRepr::U16),
                    "u32" => out.int = Some(IntRepr::U32),
                    "u64" => out.int = Some(IntRepr::U64),
                    "usize" => out.int = Some(IntRepr::Usize),
                    "i8" => out.int = Some(IntRepr::I8),
                    "i16" => out.int = Some(IntRepr::I16),
                    "i32" => out.int = Some(IntRepr::I32),
                    "i64" => out.int = Some(IntRepr::I64),
                    "isize" => out.int = Some(IntRepr::Isize),

                    // packed, align
                    "packed" => {
                        // Optional parameter in parentheses: packed or packed(n)
                        if meta.input.peek(Paren) {
                            let content;
                            syn::parenthesized!(content in meta.input);
                            let lit: LitInt = content.parse()?;
                            out.packed = Some(parse_align(lit)?);
                        } else {
                            out.packed = Some(1);
                        }
                    }
                    "align" => {
                        // align(N)
                        let content;
                        syn::parenthesized!(content in meta.input);
                        let lit: LitInt = content.parse()?;
                        out.align = Some(parse_align(lit)?);
                    }
                    _ => return Err(meta.error("unknown repr option")),
                }
            }
            Ok(())
        })?;
    }

    Ok(if found { Some(out) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn parse_repr_c_u8() {
        let attr: Attribute = parse_quote!(#[repr(C, u8)]);
        let info = parse_repr(&[attr]).unwrap().unwrap();
        assert_eq!(info.kind, Some(ReprKind::C));
        assert_eq!(info.int, Some(IntRepr::U8));
        assert_eq!(info.align, None);
        assert_eq!(info.packed, None);
    }

    #[test]
    fn parse_repr_transparent() {
        let attr: Attribute = parse_quote!(#[repr(transparent)]);
        let info = parse_repr(&[attr]).unwrap().unwrap();
        assert_eq!(info.kind, Some(ReprKind::Transparent));
    }

    #[test]
    fn parse_repr_packed_and_align() {
        let attr: Attribute = parse_quote!(#[repr(packed, align(8))]);
        let info = parse_repr(&[attr]).unwrap().unwrap();
        assert_eq!(info.packed, Some(1));
        assert_eq!(info.align, Some(8));
    }

    #[test]
    fn parse_repr_packed_n() {
        let attr: Attribute = parse_quote!(#[repr(packed(2))]);
        let info = parse_repr(&[attr]).unwrap().unwrap();
        assert_eq!(info.packed, Some(2));
    }
}
