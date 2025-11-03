//! Derives for macrokid_core::threads
//!
//! Minimal `#[derive(Job)]` prototype:
//! - Expects an inherent method `fn run(self)` on the target type by default.
//! - Optional attribute: `#[job(method = "run_impl")]` to call a different method name.
//! - Implements `macrokid_core::threads::JobRun` for the type, enabling `SpawnExt`.
//!
//! Example:
//! ```ignore
//! use std::sync::Arc;
//! use macrokid_core::threads::{ThreadPool, SpawnExt};
//! use macrokid_threads_derive::Job;
//!
//! #[derive(Clone, Job)]
//! struct Build { data: Arc<Vec<u8>> }
//! impl Build { fn run(self) { /* do work */ } }
//!
//! let pool = ThreadPool::new(4);
//! Build { data: Arc::new(vec![1,2,3]) }.spawn(&pool);
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{DeriveInput, spanned::Spanned};

#[proc_macro_derive(Job, attributes(job))]
pub fn derive_job(input: TokenStream) -> TokenStream {
    let di: DeriveInput = syn::parse(input).expect("parse derive input");
    let ident = di.ident.clone();

    // Parse optional #[job(method = "...")]
    let mut method_name: Option<syn::Ident> = None;
    for a in &di.attrs {
        if a.path().is_ident("job") {
            let parsed = a.parse_args_with(|stream: syn::parse::ParseStream| {
                let mut out: Option<syn::Ident> = None;
                while !stream.is_empty() {
                    let key: syn::Ident = stream.parse()?;
                    stream.parse::<syn::Token![=]>()?;
                    match key.to_string().as_str() {
                        "method" => {
                            let lit: syn::LitStr = stream.parse()?;
                            out = Some(syn::Ident::new(&lit.value(), Span::call_site()));
                        }
                        _ => return Err(syn::Error::new_spanned(key, "unknown key in #[job(...)]")),
                    }
                    let _ = stream.parse::<syn::Token![,]>();
                }
                Ok(out)
            });
            match parsed {
                Ok(Some(id)) => { method_name = Some(id); }
                Ok(None) => {}
                Err(e) => return e.to_compile_error().into(),
            }
        }
    }
    let method_ident = method_name.unwrap_or_else(|| syn::Ident::new("run", Span::call_site()));

    let expanded = quote! {
        impl macrokid_core::threads::JobRun for #ident {
            fn run(self) { self.#method_ident() }
        }
    };
    expanded.into()
}

#[proc_macro_derive(System, attributes(reads, writes))]
pub fn derive_system(input: TokenStream) -> TokenStream {
    let di: DeriveInput = match syn::parse(input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let ident = di.ident.clone();

    // Collect types from #[reads(T, U,...)] and #[writes(X,...)]
    fn types_from_attr(di: &DeriveInput, name: &str) -> syn::Result<Vec<syn::Type>> {
        let mut out = Vec::new();
        for a in &di.attrs {
            if a.path().is_ident(name) {
                let list: syn::punctuated::Punctuated<syn::Type, syn::Token![,]> = a.parse_args_with(
                    syn::punctuated::Punctuated::parse_terminated,
                )?;
                out.extend(list.into_iter());
            }
        }
        Ok(out)
    }

    // Check if a type is a GPU resource (GpuBuffer<T> or GpuImage<T>)
    fn is_gpu_type(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                let ident_str = segment.ident.to_string();
                return ident_str == "GpuBuffer" || ident_str == "GpuImage";
            }
        }
        false
    }

    let reads = match types_from_attr(&di, "reads") { Ok(v) => v, Err(e) => return e.to_compile_error().into() };
    let writes = match types_from_attr(&di, "writes") { Ok(v) => v, Err(e) => return e.to_compile_error().into() };

    // Separate CPU and GPU resources
    let (cpu_reads, gpu_reads): (Vec<_>, Vec<_>) = reads.iter().partition(|t| !is_gpu_type(t));
    let (cpu_writes, gpu_writes): (Vec<_>, Vec<_>) = writes.iter().partition(|t| !is_gpu_type(t));

    // Build static arrays of TypeId::of::<T>() for CPU resources
    let reads_ids: Vec<TokenStream2> = cpu_reads.iter().map(|t| quote! { ::std::any::TypeId::of::<#t>() }).collect();
    let writes_ids: Vec<TokenStream2> = cpu_writes.iter().map(|t| quote! { ::std::any::TypeId::of::<#t>() }).collect();

    // Generate GPU metadata for GPU resources
    let gpu_reads_meta: Vec<TokenStream2> = gpu_reads.iter().map(|t| {
        quote! {
            <#t as macrokid_graphics::resources::GpuResource>::metadata()
        }
    }).collect();

    let gpu_writes_meta: Vec<TokenStream2> = gpu_writes.iter().map(|t| {
        quote! {
            <#t as macrokid_graphics::resources::GpuResource>::metadata()
        }
    }).collect();

    let has_gpu_resources = !gpu_reads.is_empty() || !gpu_writes.is_empty();

    // Generate CPU ResourceAccess impl
    let resource_access_impl = quote! {
        impl macrokid_core::threads::ResourceAccess for #ident {
            fn reads() -> &'static [::std::any::TypeId] {
                static READS: ::std::sync::OnceLock<::std::vec::Vec<::std::any::TypeId>> = ::std::sync::OnceLock::new();
                READS.get_or_init(|| vec![ #( #reads_ids ),* ]).as_slice()
            }
            fn writes() -> &'static [::std::any::TypeId] {
                static WRITES: ::std::sync::OnceLock<::std::vec::Vec<::std::any::TypeId>> = ::std::sync::OnceLock::new();
                WRITES.get_or_init(|| vec![ #( #writes_ids ),* ]).as_slice()
            }
        }
    };

    // Generate GPU GpuResourceAccess impl if GPU resources are detected
    let gpu_resource_access_impl = if has_gpu_resources {
        quote! {
            impl macrokid_graphics::resources::GpuResourceAccess for #ident {
                fn gpu_reads() -> &'static [macrokid_graphics::resources::GpuResourceMeta] {
                    static GPU_READS: ::std::sync::OnceLock<::std::vec::Vec<macrokid_graphics::resources::GpuResourceMeta>> = ::std::sync::OnceLock::new();
                    GPU_READS.get_or_init(|| vec![ #( #gpu_reads_meta ),* ]).as_slice()
                }

                fn gpu_writes() -> &'static [macrokid_graphics::resources::GpuResourceMeta] {
                    static GPU_WRITES: ::std::sync::OnceLock<::std::vec::Vec<macrokid_graphics::resources::GpuResourceMeta>> = ::std::sync::OnceLock::new();
                    GPU_WRITES.get_or_init(|| vec![ #( #gpu_writes_meta ),* ]).as_slice()
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #resource_access_impl
        #gpu_resource_access_impl
    };

    expanded.into()
}

#[proc_macro_derive(Schedule, attributes(stage))]
pub fn derive_schedule(input: TokenStream) -> TokenStream {
    let di: DeriveInput = match syn::parse(input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    let ident = di.ident.clone();
    let data = match di.data { syn::Data::Struct(s) => s, _ => {
        return syn::Error::new(Span::call_site(), "Schedule derive expects a struct").to_compile_error().into()
    } };
    let fields = match data.fields { syn::Fields::Named(n) => n.named, syn::Fields::Unnamed(u) => u.unnamed, syn::Fields::Unit => {
        return syn::Error::new(Span::call_site(), "Schedule requires fields annotated with #[stage(...)]").to_compile_error().into()
    } };

    // Collect stage metadata: name, after, and job terms for each field.
    struct StageMeta {
        name: String,
        after: Vec<String>,
        before: Vec<String>,
        jobs: Vec<TokenStream2>,
        tys: Vec<syn::Type>,
    }

    let mut metas: Vec<StageMeta> = Vec::new();

    for (idx, f) in fields.iter().enumerate() {
        // parse #[stage(name = "...", after = "...")]
        let mut has_stage = false;
        let mut name_opt: Option<String> = None;
        let mut after_list: Vec<String> = Vec::new();
        let mut before_list: Vec<String> = Vec::new();
        for a in &f.attrs {
            if a.path().is_ident("stage") {
                has_stage = true;
                let parsed = a.parse_args_with(|input: syn::parse::ParseStream| {
                    while !input.is_empty() {
                        let key: syn::Ident = input.parse()?;
                        input.parse::<syn::Token![=]>()?;
                        let lit: syn::LitStr = input.parse()?;
                        match key.to_string().as_str() {
                            "name" => name_opt = Some(lit.value()),
                            "after" => {
                                // support multiple deps: "a, b, c"
                                for part in lit.value().split(',') {
                                    let s = part.trim();
                                    if !s.is_empty() { after_list.push(s.to_string()); }
                                }
                            }
                            "before" => {
                                for part in lit.value().split(',') {
                                    let s = part.trim();
                                    if !s.is_empty() { before_list.push(s.to_string()); }
                                }
                            }
                            _ => return Err(syn::Error::new_spanned(key, "unknown key in #[stage(...)]")),
                        }
                        let _ = input.parse::<syn::Token![,]>();
                    }
                    Ok(())
                });
                if let Err(e) = parsed { return e.to_compile_error().into(); }
            }
        }
        if !has_stage { continue; }

        // derive default stage name if not specified
        let name = name_opt.unwrap_or_else(|| match &f.ident {
            Some(id) => id.to_string(),
            None => format!("stage{}", idx),
        });

        // field access expression
        let field_access: TokenStream2 = match &f.ident {
            Some(id) => quote! { self.#id },
            None => { let i = syn::Index::from(idx); quote! { self.#i } },
        };

        // Expect tuple type
        let tys: Vec<syn::Type> = match &f.ty {
            syn::Type::Tuple(tt) => tt.elems.iter().cloned().collect(),
            _ => { return syn::Error::new(f.ty.span(), "#[stage] field must be a tuple of systems").to_compile_error().into() }
        };

        // Build jobs for this stage
        let mut jobs: Vec<TokenStream2> = Vec::new();
        for (i, _t) in tys.iter().enumerate() {
            let index = syn::Index::from(i);
            jobs.push(quote! {{
                let sys = #field_access.#index.clone();
                Box::new(move || macrokid_core::threads::JobRun::run(sys)) as Box<dyn FnOnce() + Send + 'static>
            }});
        }

        metas.push(StageMeta { name, after: after_list, before: before_list, jobs, tys });
    }

    // Topologically sort stages by `after` dependencies.
    let n = metas.len();
    let mut name_to_idx = std::collections::HashMap::<String, usize>::new();
    for (i, m) in metas.iter().enumerate() { name_to_idx.insert(m.name.clone(), i); }
    let mut indeg = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, m) in metas.iter().enumerate() {
        // after: edge dep -> i
        for dep in &m.after {
            let Some(&j) = name_to_idx.get(dep) else {
                return syn::Error::new(Span::call_site(), format!("unknown stage in 'after': '{}'", dep)).to_compile_error().into();
            };
            adj[j].push(i); indeg[i] += 1;
        }
        // before: edge i -> dep
        for dep in &m.before {
            let Some(&j) = name_to_idx.get(dep) else {
                return syn::Error::new(Span::call_site(), format!("unknown stage in 'before': '{}'", dep)).to_compile_error().into();
            };
            adj[i].push(j); indeg[j] += 1;
        }
    }
    // Kahn's algorithm, preserving declaration order among zero-indegree nodes
    let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut order: Vec<usize> = Vec::with_capacity(n);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] { indeg[v] -= 1; if indeg[v] == 0 { queue.push_back(v); } }
    }
    if order.len() != n {
        return syn::Error::new(Span::call_site(), "cycle detected in #[stage(after = ...)] graph").to_compile_error().into();
    }

    // Emit blocks in sorted order
    let stage_blocks: Vec<TokenStream2> = order.into_iter().map(|i| {
        let jobs = &metas[i].jobs;
        let tys = &metas[i].tys;
        let n_jobs = jobs.len();
        quote! {
            // Conflict-aware batching within stage using ResourceAccess
            let reads: [&[::std::any::TypeId]; #n_jobs] = [ #( <#tys as macrokid_core::threads::ResourceAccess>::reads() ),* ];
            let writes: [&[::std::any::TypeId]; #n_jobs] = [ #( <#tys as macrokid_core::threads::ResourceAccess>::writes() ),* ];
            let mut remaining: ::std::vec::Vec<usize> = (0..#n_jobs).collect();
            let mut jobs: ::std::vec::Vec<::std::option::Option<macrokid_core::threads::Job>> = ::std::vec::Vec::with_capacity(#n_jobs);
            #( jobs.push(Some(#jobs)); )*
            while !remaining.is_empty() {
                let mut layer: ::std::vec::Vec<usize> = ::std::vec::Vec::new();
                let snapshot = remaining.clone();
                for i in snapshot {
                    let mut ok = true;
                    for &j in &layer {
                        // check conflicts between i and j
                        // conflict if writes[i]∩writes[j] or writes[i]∩reads[j] or writes[j]∩reads[i]
                        let wr_i = writes[i]; let wr_j = writes[j]; let rd_i = reads[i]; let rd_j = reads[j];
                        let mut conflict = false;
                        'a: {
                            for a in wr_i { for b in wr_j { if a == b { conflict = true; break 'a; } } }
                            for a in wr_i { for b in rd_j { if a == b { conflict = true; break 'a; } } }
                            for a in wr_j { for b in rd_i { if a == b { conflict = true; break 'a; } } }
                        }
                        if conflict { ok = false; break; }
                    }
                    if ok { layer.push(i); }
                }
                remaining.retain(|x| !layer.contains(x));
                let batch: ::std::vec::Vec<_> = layer.into_iter().map(|k| jobs[k].take().unwrap()).collect();
                macrokid_core::threads::join_all(sched, batch);
            }
        }
    }).collect();

    // Prepare constants for a debug grouping method
    let name_literals: Vec<TokenStream2> = metas.iter().map(|m| {
        let s = m.name.clone();
        quote! { #s }
    }).collect();
    let mut edge_pairs: Vec<(usize, usize)> = Vec::new();
    for (i, m) in metas.iter().enumerate() {
        for dep in &m.after { let &j = name_to_idx.get(dep).unwrap(); edge_pairs.push((j, i)); }
        for dep in &m.before { let &j = name_to_idx.get(dep).unwrap(); edge_pairs.push((i, j)); }
    }
    let edge_terms: Vec<TokenStream2> = edge_pairs.iter().map(|(u, v)| {
        let uu = syn::Index::from(*u); let vv = syn::Index::from(*v);
        quote! { (#uu as usize, #vv as usize) }
    }).collect();

    let expanded = quote! {
        impl #ident {
            pub fn run<S: macrokid_core::threads::Scheduler>(&self, sched: &S) {
                #( #stage_blocks )*
            }

            /// Return topological groups (layers) of stages for debugging.
            pub fn topo_groups() -> ::std::vec::Vec<::std::vec::Vec<&'static str>> {
                let names: [&'static str; #n] = [ #( #name_literals ),* ];
                let edges: &[(usize, usize)] = &[ #( #edge_terms ),* ];
                let mut indeg = vec![0usize; #n];
                let mut adj: ::std::vec::Vec<::std::vec::Vec<usize>> = vec![::std::vec::Vec::new(); #n];
                for (u, v) in edges { adj[*u].push(*v); indeg[*v] += 1; }
                use ::std::collections::VecDeque;
                let mut cur: VecDeque<usize> = (0..#n).filter(|&i| indeg[i] == 0).collect();
                let mut groups: ::std::vec::Vec<::std::vec::Vec<&'static str>> = ::std::vec::Vec::new();
                while !cur.is_empty() {
                    let mut next = VecDeque::new();
                    let mut layer = ::std::vec::Vec::new();
                    while let Some(u) = cur.pop_front() {
                        layer.push(names[u]);
                        for &v in &adj[u] {
                            indeg[v] -= 1;
                            if indeg[v] == 0 { next.push_back(v); }
                        }
                    }
                    groups.push(layer);
                    cur = next;
                }
                groups
            }
        }
    };
    expanded.into()
}
