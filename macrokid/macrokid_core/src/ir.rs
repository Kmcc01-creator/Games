use proc_macro2::Span;
use syn::{spanned::Spanned, Attribute, Data, DeriveInput, Field, Fields, Generics, Ident, Type, Visibility, Expr};

#[derive(Debug, Clone)]
pub struct TypeSpec {
    pub ident: Ident,
    pub generics: Generics,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub span: Span,
    pub kind: TypeKind,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct(StructSpec),
    Enum(EnumSpec),
}

#[derive(Debug, Clone)]
pub struct StructSpec {
    pub fields: FieldKind,
}

#[derive(Debug, Clone)]
pub struct EnumSpec {
    pub variants: Vec<VariantSpec>,
}

#[derive(Debug, Clone)]
pub struct VariantSpec {
    pub ident: Ident,
    pub attrs: Vec<Attribute>,
    pub span: Span,
    pub discriminant: Option<Expr>,
    pub fields: FieldKind,
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    Named(Vec<FieldSpec>),
    Unnamed(Vec<FieldSpec>),
    Unit,
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub ident: Option<Ident>,
    pub index: usize,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ty: Type,
    pub span: Span,
}

impl TypeSpec {
    /// Returns true if this is a struct
    pub fn is_struct(&self) -> bool { matches!(self.kind, TypeKind::Struct(_)) }
    /// Returns true if this is an enum
    pub fn is_enum(&self) -> bool { matches!(self.kind, TypeKind::Enum(_)) }
    /// Borrow as StructSpec if struct
    pub fn as_struct(&self) -> Option<&StructSpec> { if let TypeKind::Struct(ref s) = self.kind { Some(s) } else { None } }
    /// Borrow as EnumSpec if enum
    pub fn as_enum(&self) -> Option<&EnumSpec> { if let TypeKind::Enum(ref e) = self.kind { Some(e) } else { None } }
    pub fn from_derive_input(input: DeriveInput) -> syn::Result<Self> {
        let span = input.ident.span();
        let ident = input.ident;
        let generics = input.generics;
        let attrs = input.attrs;
        let vis = input.vis;
        let kind = match input.data {
            Data::Struct(s) => TypeKind::Struct(StructSpec {
                fields: FieldKind::from_fields(s.fields)?,
            }),
            Data::Enum(e) => {
                let mut variants = Vec::with_capacity(e.variants.len());
                for v in e.variants {
                    let fields = FieldKind::from_fields(v.fields)?;
                    let discriminant = v.discriminant.map(|(_, expr)| expr);
                    variants.push(VariantSpec {
                        ident: v.ident,
                        attrs: v.attrs,
                        span: span_for_variant(&fields),
                        discriminant,
                        fields,
                    });
                }
                TypeKind::Enum(EnumSpec { variants })
            }
            Data::Union(u) => {
                return Err(syn::Error::new(u.union_token.span(), "unions are not supported"));
            }
        };

        Ok(Self {
            ident,
            generics,
            attrs,
            vis,
            span,
            kind,
        })
    }
}

fn span_for_variant(fields: &FieldKind) -> Span {
    match fields {
        FieldKind::Named(v) | FieldKind::Unnamed(v) => v
            .first()
            .map(|f| f.span)
            .unwrap_or_else(Span::call_site),
        FieldKind::Unit => Span::call_site(),
    }
}

impl FieldKind {
    pub fn from_fields(fields: Fields) -> syn::Result<Self> {
        Ok(match fields {
            Fields::Named(named) => {
                let specs = named
                    .named
                    .into_iter()
                    .enumerate()
                    .map(|(i, f)| FieldSpec::from_field(f, i))
                    .collect();
                FieldKind::Named(specs)
            }
            Fields::Unnamed(unnamed) => {
                let specs = unnamed
                    .unnamed
                    .into_iter()
                    .enumerate()
                    .map(|(i, f)| FieldSpec::from_field(f, i))
                    .collect();
                FieldKind::Unnamed(specs)
            }
            Fields::Unit => FieldKind::Unit,
        })
    }
}

impl FieldSpec {
    fn from_field(field: Field, index: usize) -> Self {
        let span = field.span();
        let ty = field.ty.clone();
        Self { ident: field.ident, index, attrs: field.attrs, vis: field.vis, ty, span }
    }
}

impl StructSpec {
    pub fn fields(&self) -> &FieldKind { &self.fields }
    pub fn field_count(&self) -> usize {
        match &self.fields {
            FieldKind::Named(v) | FieldKind::Unnamed(v) => v.len(),
            FieldKind::Unit => 0,
        }
    }
    pub fn is_unit(&self) -> bool { matches!(self.fields, FieldKind::Unit) }
}

impl EnumSpec {
    pub fn variants(&self) -> &[VariantSpec] { &self.variants }
    pub fn variant_count(&self) -> usize { self.variants.len() }
    pub fn has_tuple_variants(&self) -> bool {
        self.variants.iter().any(|v| matches!(v.fields, FieldKind::Unnamed(_)))
    }
    /// Collect fields of all variants for quick scans
    pub fn fields_of_variants(&self) -> Vec<&FieldKind> { self.variants.iter().map(|v| &v.fields).collect() }
}
