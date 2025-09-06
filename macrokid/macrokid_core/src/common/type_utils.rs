use syn::{GenericArgument, PathArguments, Type};

fn single_generic_arg(path: &syn::Path) -> Option<&GenericArgument> {
    path.segments.last().and_then(|seg| match &seg.arguments {
        PathArguments::AngleBracketed(ab) => ab.args.first(),
        _ => None,
    })
}

fn last_ident_is(path: &syn::Path, name: &str) -> bool {
    path.segments.last().is_some_and(|seg| seg.ident == name)
}

/// Returns true if type is `Option<T>`
pub fn is_option(ty: &Type) -> bool {
    if let Type::Path(tp) = ty { last_ident_is(&tp.path, "Option") } else { false }
}

/// If type is `Option<T>`, return `T`
pub fn unwrap_option(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if last_ident_is(&tp.path, "Option") {
            return single_generic_arg(&tp.path).and_then(|ga| match ga { GenericArgument::Type(t) => Some(t), _ => None });
        }
    }
    None
}

/// Returns true if type is `Vec<T>`
pub fn is_vec(ty: &Type) -> bool {
    if let Type::Path(tp) = ty { last_ident_is(&tp.path, "Vec") } else { false }
}

/// If type is `Vec<T>`, return `T`
pub fn unwrap_vec(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if last_ident_is(&tp.path, "Vec") {
            return single_generic_arg(&tp.path).and_then(|ga| match ga { GenericArgument::Type(t) => Some(t), _ => None });
        }
    }
    None
}

/// Returns Some((T, E)) if type is `Result<T, E>`
pub fn unwrap_result(ty: &Type) -> Option<(&Type, &Type)> {
    if let Type::Path(tp) = ty {
        if last_ident_is(&tp.path, "Result") {
            if let Some(PathArguments::AngleBracketed(ab)) = tp.path.segments.last().map(|s| &s.arguments) {
                let mut it = ab.args.iter().filter_map(|ga| if let GenericArgument::Type(t) = ga { Some(t) } else { None });
                if let (Some(t), Some(e)) = (it.next(), it.next()) { return Some((t, e)); }
            }
        }
    }
    None
}

/// Returns true if type is `Box<T>`
pub fn is_box(ty: &Type) -> bool {
    if let Type::Path(tp) = ty { last_ident_is(&tp.path, "Box") } else { false }
}

/// If type is `Box<T>`, return `T`
pub fn unwrap_box(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if last_ident_is(&tp.path, "Box") {
            return single_generic_arg(&tp.path).and_then(|ga| match ga { GenericArgument::Type(t) => Some(t), _ => None });
        }
    }
    None
}

/// Returns true if type is `PhantomData<T>`
pub fn is_phantom_data(ty: &Type) -> bool {
    if let Type::Path(tp) = ty { last_ident_is(&tp.path, "PhantomData") } else { false }
}

/// If type is `PhantomData<T>`, return `T`
pub fn unwrap_phantom_data(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if last_ident_is(&tp.path, "PhantomData") {
            return single_generic_arg(&tp.path).and_then(|ga| match ga { GenericArgument::Type(t) => Some(t), _ => None });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn option_helpers() {
        let ty: Type = parse_quote!(Option<String>);
        assert!(is_option(&ty));
        assert!(unwrap_option(&ty).is_some());
    }

    #[test]
    fn vec_helpers() {
        let ty: Type = parse_quote!(Vec<u8>);
        assert!(is_vec(&ty));
        assert!(unwrap_vec(&ty).is_some());
    }

    #[test]
    fn result_helpers() {
        let ty: Type = parse_quote!(Result<u32, E>);
        let (ok, err) = unwrap_result(&ty).expect("unwrap result");
        match ok { Type::Path(_) => {}, _ => panic!("expected path") }
        match err { Type::Path(_) => {}, _ => panic!("expected path") }
    }

    #[test]
    fn box_helpers() {
        let ty: Type = parse_quote!(Box<String>);
        assert!(is_box(&ty));
        assert!(unwrap_box(&ty).is_some());
    }

    #[test]
    fn phantom_data_helpers() {
        let ty: Type = parse_quote!(::core::marker::PhantomData<MyT>);
        assert!(is_phantom_data(&ty));
        assert!(unwrap_phantom_data(&ty).is_some());
    }
}
