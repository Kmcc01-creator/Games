use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput};
use macrokid_parse_bench::parse_resource_attr;

// Sample input: a struct with typical macrokid attributes
fn sample_input() -> TokenStream {
    quote! {
        #[derive(ResourceBinding)]
        struct Material {
            #[uniform(set = 0, binding = 0, stages = "vs|fs")]
            matrices: Mat4,

            #[texture(set = 0, binding = 1, stages = "fs")]
            albedo: Texture2D,

            #[sampler(set = 0, binding = 2, stages = "fs")]
            sampler: Sampler,

            #[combined(set = 0, binding = 3, stages = "fs")]
            combined: CombinedImageSampler,
        }
    }
}

// Baseline: Full syn parsing
fn bench_syn_full_parse(c: &mut Criterion) {
    let input = sample_input();

    c.bench_function("syn_full_parse", |b| {
        b.iter(|| {
            let derive_input: DeriveInput = syn::parse2(black_box(input.clone())).unwrap();
            black_box(derive_input)
        })
    });
}

// Syn: Parse just attributes
fn bench_syn_attrs_only(c: &mut Criterion) {
    let input = sample_input();
    let derive_input: DeriveInput = syn::parse2(input.clone()).unwrap();

    c.bench_function("syn_parse_attrs", |b| {
        b.iter(|| {
            let attrs: Vec<&Attribute> = derive_input
                .attrs
                .iter()
                .filter(|a| {
                    a.path().is_ident("uniform")
                        || a.path().is_ident("texture")
                        || a.path().is_ident("sampler")
                        || a.path().is_ident("combined")
                })
                .collect();
            black_box(attrs)
        })
    });
}

// Syn: Parse nested meta (what we actually do)
fn bench_syn_parse_nested_meta(c: &mut Criterion) {
    let input = sample_input();
    let derive_input: DeriveInput = syn::parse2(input.clone()).unwrap();

    c.bench_function("syn_parse_nested_meta", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for attr in &derive_input.attrs {
                if attr.path().is_ident("uniform") {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("set") {
                            let value: syn::LitInt = meta.value()?.parse()?;
                            results.push(("set", value.base10_parse::<u32>().unwrap()));
                        } else if meta.path.is_ident("binding") {
                            let value: syn::LitInt = meta.value()?.parse()?;
                            results.push(("binding", value.base10_parse::<u32>().unwrap()));
                        }
                        Ok(())
                    });
                }
            }
            black_box(results)
        })
    });
}

// Custom parser: Our fast attribute parser
fn bench_custom_parse(c: &mut Criterion) {
    let tokens = quote! {
        #[uniform(set = 0, binding = 1, stages = "vs|fs")]
    };

    c.bench_function("custom_parse", |b| {
        b.iter(|| {
            let result = parse_resource_attr(black_box(tokens.clone())).unwrap();
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_syn_full_parse,
    bench_syn_attrs_only,
    bench_syn_parse_nested_meta,
    bench_custom_parse
);
criterion_main!(benches);
