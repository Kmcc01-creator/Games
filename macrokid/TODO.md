# Macrokid TODO List

## High Priority

### 🔴 Fix Assets Module Proc Macro Location
**Issue**: Proc macro derives must be at crate root, but currently in module

**Error**:
```
error: functions tagged with `#[proc_macro_derive]` must currently reside in the root of the crate
```

**Location**: `macrokid_graphics_derive/src/assets.rs`

**Affected Derives**:
- `ProceduralMesh` (line 39)
- `ProceduralTexture` (line 294)
- `AssetBundle` (line 330)

**Solution**:
1. Move `derive_entry!` macro calls to `macrokid_graphics_derive/src/lib.rs`
2. Keep helper functions (`expand_procedural_mesh`, etc.) in assets.rs
3. Import helpers from assets module

**Example**:
```rust
// In lib.rs (root):
mod assets;
use assets::{expand_procedural_mesh, expand_procedural_texture, expand_asset_bundle};

derive_entry!(ProceduralMesh, attrs = [primitive, transform, material], handler = expand_procedural_mesh);
derive_entry!(ProceduralTexture, attrs = [texture, noise, gradient], handler = expand_procedural_texture);
derive_entry!(AssetBundle, attrs = [mesh, texture], handler = expand_asset_bundle);

// In assets.rs:
// Remove derive_entry! calls, keep only:
pub fn expand_procedural_mesh(input: DeriveInput) -> syn::Result<TokenStream> { ... }
pub fn expand_procedural_texture(input: DeriveInput) -> syn::Result<TokenStream> { ... }
pub fn expand_asset_bundle(input: DeriveInput) -> syn::Result<TokenStream> { ... }
```

**Status**: Module currently disabled (see `lib.rs:17`)

**Estimated Effort**: 30 minutes

---

## Medium Priority

### 🟡 Apply StaticSliceDerive to Remaining Derives

**Candidates**:
- `GraphicsPipeline` - Currently uses custom CodeGen pattern
- `RenderPass` - Type-level derive (may not fit pattern)
- `RenderEngine` - Type-level derive (may not fit pattern)

**Benefits**:
- Reduce boilerplate
- Consistent patterns across derives
- Easier maintenance

**Location**: `macrokid_graphics_derive/src/lib.rs:389-791`

**Status**: Not started

**Estimated Effort**: 2-3 hours

---

### 🟡 Clean Up Unused Variable Warnings

**Files**:
- `macrokid_graphics_derive/src/lib.rs` (4 warnings)

**Warnings**:
- `ct_entries_tokens` (line 542)
- `ct_mod_ident_opt` (line 733)
- `ct_mod_tokens_opt` (line 733)
- Various others

**Solution**: Prefix with underscore or remove if truly unused

**Status**: Not started

**Estimated Effort**: 15 minutes

---

## Low Priority

### 🟢 Optimize Code Generation Performance

**Analysis**: Parsing is only 10-20% of macro expansion time

**Opportunities**:
1. Quote! optimization - Reduce token stream allocations
2. Caching - Reuse generated descriptors across macro invocations
3. Incremental generation - Only regenerate changed derives

**Reference**: See `PARSE_BENCHMARK_ANALYSIS.md` for details

**Status**: Research phase

**Estimated Effort**: Unknown (requires profiling)

---

### 🟢 Add More Examples

**Needed**:
- Advanced PBR shader example (exists: `examples/pbr_showcase.rs`)
- Deferred lighting example (spec exists: `examples/deferred_clustered_lighting.md`)
- Asset bundle example using `AssetBundle` derive
- Texture generation examples

**Status**: Partially complete

**Estimated Effort**: 1-2 hours per example

---

### 🟢 Add Tests for Float Support

**New Features**:
- `AttrSchema::opt_float()` and `req_float()`
- `ParsedAttrs::get_float()` and `try_get_float()`

**Location**: `macrokid_core/src/common/attr_schema.rs:160+`

**Needed Tests**:
```rust
#[test]
fn parse_float_schema() {
    let schema = AttrSchema::new("primitive")
        .opt_float("size")
        .opt_float("radius");
    let attr: Attribute = parse_quote!(#[primitive(size = 1.5, radius = 2.0)]);
    let res = schema.parse(&[attr]).expect("ok");
    assert_eq!(res.get_float("size"), Some(1.5));
    assert_eq!(res.get_float("radius"), Some(2.0));
}
```

**Status**: Not started

**Estimated Effort**: 30 minutes

---

## Completed ✅

- ✅ Add float support to AttrSchema API
- ✅ Refactor BufferLayout derive (161→91 lines, 43% reduction)
- ✅ Benchmark custom vs syn attribute parsing
- ✅ Fix glam dependency issues
- ✅ Add Vertex trait bounds to mesh generation functions
- ✅ Fix quote! macro issues in assets module
- ✅ Create comprehensive documentation (PARSE_BENCHMARK_ANALYSIS.md, RECENT_UPDATES.md)

---

## Documentation

### Existing Docs
- ✅ `OPTION_B_COMPLETE.md` - Framework features summary
- ✅ `CODEGEN_TUTORIAL.md` - Code generation tutorial
- ✅ `CODEGEN_QUICKREF.md` - Quick reference
- ✅ `FRAMEWORK_IMPROVEMENTS.md` - Architecture overview
- ✅ `PARSE_BENCHMARK_ANALYSIS.md` - Benchmarking analysis
- ✅ `RECENT_UPDATES.md` - Latest session updates

### Needed Docs
- 🟡 API reference for all derives
- 🟡 Migration guide for old code
- 🟡 Performance tuning guide

---

## Notes

- **Performance**: Attribute parsing is well-optimized (see benchmarks). Focus on code generation if optimization needed.
- **Patterns**: StaticSliceDerive works well for field-based derives. Type-level derives may need different patterns.
- **Testing**: All core functionality has tests. New features (float support) need test coverage.
- **syn/quote deps**: `deps/syn` and `deps/quote` are currently cloned git repos (not submodules). Need to decide: convert to submodules, vendor the source (remove .git), use cargo deps, or remove entirely. Currently unstaged from commit.
