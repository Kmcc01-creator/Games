//! Asset Generation Derive Macros
//!
//! This module provides implementation functions for asset derive macros:
//! - `ProceduralMesh` - Generate geometry at compile-time
//! - `ProceduralTexture` - Generate textures at compile-time
//! - `AssetBundle` - Combine multiple assets with metadata
//!
//! ## Organization
//!
//! Following Rust proc macro requirements, the `#[proc_macro_derive]` entry points
//! are defined in `lib.rs` (crate root), while the implementation logic resides here.
//! This is the standard pattern for organizing proc macro crates.

use proc_macro2::Span;
use macrokid_core::{
    ir::{TypeSpec, FieldKind},
    attr_schema::AttrSchema,
};
use quote::quote;
use syn::DeriveInput;

// ==================== ProceduralMesh Derive ====================

pub fn expand_procedural_mesh(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();

    // Parse type-level primitive specification
    let primitive_schema = AttrSchema::new("primitive")
        .req_str("type")        // sphere, cube, plane, cylinder
        .opt_float("size")      // general size parameter
        .opt_float("radius")    // for spheres/cylinders  
        .opt_float("width")     // for planes/cubes
        .opt_float("height")    // for planes/cylinders
        .opt_float("depth")     // for cubes
        .opt_int("segments")    // tessellation level
        .opt_int("rings")       // for spheres
        .opt_int("sectors");    // for spheres

    let primitive_attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &primitive_schema)?;
    
    let prim_type = primitive_attrs.try_get_str("type")?;
    let generation_code = match prim_type {
        "sphere" => {
            let radius = primitive_attrs.get_float("radius")
                .or_else(|| primitive_attrs.get_float("size"))
                .unwrap_or(1.0);
            let rings = primitive_attrs.get_int("rings").unwrap_or(16) as u32;
            let sectors = primitive_attrs.get_int("sectors").unwrap_or(32) as u32;
            
            quote! {
                macrokid_graphics::assets::Primitives::uv_sphere::<macrokid_graphics::assets::SimpleVertex>(#radius, #sectors, #rings)
            }
        },
        "cube" => {
            let size = primitive_attrs.get_float("size")
                .or_else(|| primitive_attrs.get_float("width"))
                .unwrap_or(2.0);
            
            quote! {
                macrokid_graphics::assets::Primitives::cube::<macrokid_graphics::assets::SimpleVertex>(#size)
            }
        },
        "plane" => {
            let width = primitive_attrs.get_float("width").unwrap_or(4.0);
            let height = primitive_attrs.get_float("height")
                .or_else(|| primitive_attrs.get_float("depth"))
                .unwrap_or(4.0);
            let segments = primitive_attrs.get_int("segments").unwrap_or(1) as u32;
            
            quote! {
                macrokid_graphics::assets::Primitives::plane::<macrokid_graphics::assets::SimpleVertex>(#width, #height, #segments, #segments)
            }
        },
        "cylinder" => {
            let radius = primitive_attrs.get_float("radius")
                .or_else(|| primitive_attrs.get_float("size"))
                .unwrap_or(1.0);
            let height = primitive_attrs.get_float("height").unwrap_or(2.0);
            let segments = primitive_attrs.get_int("segments").unwrap_or(16) as u32;
            
            quote! {
                macrokid_graphics::assets::Primitives::cylinder::<macrokid_graphics::assets::SimpleVertex>(#radius, #height, #segments)
            }
        },
        other => return Err(syn::Error::new(spec.span, format!("unknown primitive type '{}': expected sphere|cube|plane|cylinder", other))),
    };

    // Optional transform attributes
    let transform_schema = AttrSchema::new("transform")
        .opt_str("translate")   // "1.0,2.0,3.0"
        .opt_str("rotate")      // "45,0,0" (degrees)
        .opt_str("scale");      // "1.0,1.0,1.0" or single value

    let transform_attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &transform_schema)?;
    let transform_code = if transform_attrs.map.is_empty() {
        quote! { mesh }
    } else {
        // Parse transform parameters and generate transformation code
        let mut transforms = Vec::new();
        
        if let Some(translate_str) = transform_attrs.get_str("translate") {
            let coords: Result<Vec<f32>, _> = translate_str.split(',')
                .map(|s| s.trim().parse())
                .collect();
            if let Ok(coords) = coords {
                if coords.len() == 3 {
                    let x = coords[0];
                    let y = coords[1]; 
                    let z = coords[2];
                    transforms.push(quote! {
                        mesh = macrokid_graphics::assets::transform::translate_mesh(mesh, glam::Vec3::new(#x, #y, #z));
                    });
                }
            }
        }
        
        if let Some(rotate_str) = transform_attrs.get_str("rotate") {
            let angles: Result<Vec<f32>, _> = rotate_str.split(',')
                .map(|s| s.trim().parse())
                .collect();
            if let Ok(angles) = angles {
                if angles.len() == 3 {
                    let x = angles[0] * std::f32::consts::PI / 180.0; // Convert to radians
                    let y = angles[1] * std::f32::consts::PI / 180.0;
                    let z = angles[2] * std::f32::consts::PI / 180.0;
                    transforms.push(quote! {
                        mesh = macrokid_graphics::assets::transform::rotate_mesh(mesh, glam::Vec3::new(#x, #y, #z));
                    });
                }
            }
        }
        
        if let Some(scale_str) = transform_attrs.get_str("scale") {
            let scale_parts: Result<Vec<f32>, _> = scale_str.split(',')
                .map(|s| s.trim().parse())
                .collect();
            if let Ok(parts) = scale_parts {
                let scale_vec = if parts.len() == 1 {
                    let s = parts[0];
                    quote! { glam::Vec3::splat(#s) }
                } else if parts.len() == 3 {
                    let x = parts[0];
                    let y = parts[1];
                    let z = parts[2];
                    quote! { glam::Vec3::new(#x, #y, #z) }
                } else {
                    quote! { glam::Vec3::ONE }
                };
                transforms.push(quote! {
                    mesh = macrokid_graphics::assets::transform::scale_mesh(mesh, #scale_vec);
                });
            }
        }

        if transforms.is_empty() {
            quote! { mesh }
        } else {
            quote! {
                {
                    let mut mesh = mesh;
                    #(#transforms)*
                    mesh
                }
            }
        }
    };

    // Generate the implementation
    let mod_ident = syn::Ident::new(&format!("__mk_pmesh_{}", ident), Span::call_site());
    let output = quote! {
        #[allow(non_snake_case)]
        mod #mod_ident {
            use super::*;
            
            pub fn generate_mesh() -> macrokid_graphics::assets::Mesh<macrokid_graphics::assets::SimpleVertex> {
                let mesh = #generation_code;
                #transform_code
            }
            
            // Cached static mesh - generated once
            ::std::sync::LazyLock::new(|| generate_mesh());
            pub static MESH: ::std::sync::LazyLock<macrokid_graphics::assets::Mesh<macrokid_graphics::assets::SimpleVertex>> = MESH_LAZY;
        }
        
        impl macrokid_graphics::assets::MeshProvider for #ident {
            type Vertex = macrokid_graphics::assets::SimpleVertex;
            
            fn mesh() -> &'static macrokid_graphics::assets::Mesh<Self::Vertex> {
                &#mod_ident::MESH
            }
        }
        
        impl #ident {
            pub fn generate_mesh() -> macrokid_graphics::assets::Mesh<macrokid_graphics::assets::SimpleVertex> {
                #mod_ident::generate_mesh()
            }
            
            pub fn mesh() -> &'static macrokid_graphics::assets::Mesh<macrokid_graphics::assets::SimpleVertex> {
                <Self as macrokid_graphics::assets::MeshProvider>::mesh()
            }
        }
    };

    Ok(output)
}

// ==================== ProceduralTexture Derive ====================

pub fn expand_procedural_texture(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();

    // Parse texture specification
    let texture_schema = AttrSchema::new("texture")
        .req_str("type")        // solid, checkerboard, gradient, noise, pbr_set
        .opt_int("width")       // texture dimensions
        .opt_int("height")
        .opt_str("format");     // RGBA8, RGB8, etc.

    let texture_attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &texture_schema)?;
    
    let tex_type = texture_attrs.try_get_str("type")?;
    let width = texture_attrs.get_int("width").unwrap_or(512) as u32;
    let height = texture_attrs.get_int("height").unwrap_or(512) as u32;

    let generation_code = match tex_type {
        "solid" => {
            // Look for color specification - could be in separate schema
            quote! {
                macrokid_graphics::assets::TextureGenerator::solid_color(
                    #width, #height, 
                    glam::Vec4::new(0.8, 0.2, 0.2, 1.0) // Default red
                )
            }
        },
        "checkerboard" => {
            quote! {
                macrokid_graphics::assets::TextureGenerator::checkerboard(
                    #width, #height, #width / 8,
                    glam::Vec4::new(0.9, 0.9, 0.9, 1.0),
                    glam::Vec4::new(0.1, 0.1, 0.1, 1.0)
                )
            }
        },
        "gradient" => {
            quote! {
                macrokid_graphics::assets::TextureGenerator::gradient(
                    #width, #height,
                    glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
                    glam::Vec4::new(0.0, 0.0, 1.0, 1.0),
                    true
                )
            }
        },
        "noise" => {
            // Parse noise parameters
            let noise_schema = AttrSchema::new("noise")
                .opt_float("scale")
                .opt_int("octaves");
            
            let noise_attrs = macrokid_core::common::attr_schema::scope::on_type(&spec, &noise_schema)?;
            let scale = noise_attrs.get_float("scale").unwrap_or(4.0);
            let octaves = noise_attrs.get_int("octaves").unwrap_or(3) as u32;
            
            quote! {
                macrokid_graphics::assets::TextureGenerator::perlin_noise(#width, #height, #scale, #octaves)
            }
        },
        other => return Err(syn::Error::new(spec.span, format!("unknown texture type '{}': expected solid|checkerboard|gradient|noise", other))),
    };

    let mod_ident = syn::Ident::new(&format!("__mk_ptex_{}", ident), Span::call_site());
    let output = quote! {
        #[allow(non_snake_case)]
        mod #mod_ident {
            use super::*;
            
            pub fn generate_texture() -> macrokid_graphics::assets::Texture2D {
                #generation_code
            }
            
            // Cached static texture
            static TEXTURE_LAZY: ::std::sync::LazyLock<macrokid_graphics::assets::Texture2D> = ::std::sync::LazyLock::new(|| generate_texture());
            pub static TEXTURE: ::std::sync::LazyLock<macrokid_graphics::assets::Texture2D> = TEXTURE_LAZY;
        }
        
        impl macrokid_graphics::assets::TextureProvider for #ident {
            fn texture() -> &'static macrokid_graphics::assets::Texture2D {
                &#mod_ident::TEXTURE
            }
        }
        
        impl #ident {
            pub fn generate_texture() -> macrokid_graphics::assets::Texture2D {
                #mod_ident::generate_texture()
            }
            
            pub fn texture() -> &'static macrokid_graphics::assets::Texture2D {
                <Self as macrokid_graphics::assets::TextureProvider>::texture()
            }
        }
    };

    Ok(output)
}

// ==================== AssetBundle Derive ====================

pub fn expand_asset_bundle(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let spec = TypeSpec::from_derive_input(input)?;
    let ident = spec.ident.clone();

    // Parse field references to other asset types
    let st = match &spec.kind {
        macrokid_core::TypeKind::Struct(st) => st,
        _ => return Err(syn::Error::new(spec.span, "AssetBundle expects a struct")),
    };

    let mesh_schema = AttrSchema::new("mesh_ref");
    let texture_schema = AttrSchema::new("texture_ref");

    #[derive(Clone, Debug)]
    struct AssetRef {
        field_name: String,
        field_type: proc_macro2::TokenStream,
        asset_kind: String, // "mesh" or "texture"
    }

    let mut asset_refs = Vec::new();

    match st.fields() {
        FieldKind::Named(fields) => {
            for field in fields {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let field_type = &field.ty;
                
                if mesh_schema.parse(&field.attrs).is_ok() {
                    asset_refs.push(AssetRef {
                        field_name,
                        field_type: quote! { #field_type },
                        asset_kind: "mesh".to_string(),
                    });
                } else if texture_schema.parse(&field.attrs).is_ok() {
                    asset_refs.push(AssetRef {
                        field_name,
                        field_type: quote! { #field_type },
                        asset_kind: "texture".to_string(),
                    });
                }
            }
        },
        _ => return Err(syn::Error::new(spec.span, "AssetBundle expects named fields")),
    }

    // Generate bundle accessor methods
    let accessors: Vec<proc_macro2::TokenStream> = asset_refs.iter().map(|asset_ref| {
        let method_name = syn::Ident::new(&format!("get_{}", asset_ref.field_name), Span::call_site());
        let field_type = &asset_ref.field_type;
        
        match asset_ref.asset_kind.as_str() {
            "mesh" => quote! {
                pub fn #method_name() -> &'static macrokid_graphics::assets::Mesh<macrokid_graphics::assets::SimpleVertex> {
                    <#field_type as macrokid_graphics::assets::MeshProvider>::mesh()
                }
            },
            "texture" => quote! {
                pub fn #method_name() -> &'static macrokid_graphics::assets::Texture2D {
                    <#field_type as macrokid_graphics::assets::TextureProvider>::texture()
                }
            },
            _ => quote! {},
        }
    }).collect();

    let asset_count = asset_refs.len();
    let asset_names: Vec<_> = asset_refs.iter().map(|r| r.field_name.as_str()).collect();

    let output = quote! {
        impl macrokid_graphics::assets::BundleProvider for #ident {
            fn asset_count() -> usize { #asset_count }
        }

        impl #ident {
            #(#accessors)*

            pub fn list_assets() -> Vec<&'static str> {
                vec![#(#asset_names),*]
            }
        }
    };

    Ok(output)
}

// Helper trait definitions that would go in assets.rs
pub fn generate_asset_traits() -> proc_macro2::TokenStream {
    quote! {
        /// Trait for types that provide procedural meshes
        pub trait MeshProvider {
            type Vertex: Vertex;
            fn mesh() -> &'static Mesh<Self::Vertex>;
        }
        
        /// Trait for types that provide procedural textures
        pub trait TextureProvider {
            fn texture() -> &'static Texture2D;
        }
        
        /// Trait for asset bundle types
        pub trait BundleProvider {
            fn asset_count() -> usize;
        }
        
        /// Mesh transformation utilities
        pub mod transform {
            use super::*;
            use glam::{Vec3, Mat4, Quat};
            
            pub fn translate_mesh<V: Vertex>(mut mesh: Mesh<V>, translation: Vec3) -> Mesh<V> {
                // Implementation would transform vertex positions
                // This is a placeholder - real implementation would need vertex position access
                mesh
            }
            
            pub fn rotate_mesh<V: Vertex>(mut mesh: Mesh<V>, rotation: Vec3) -> Mesh<V> {
                // Implementation would apply rotation matrix to positions and normals
                mesh
            }
            
            pub fn scale_mesh<V: Vertex>(mut mesh: Mesh<V>, scale: Vec3) -> Mesh<V> {
                // Implementation would scale vertex positions
                mesh
            }
        }
    }
}