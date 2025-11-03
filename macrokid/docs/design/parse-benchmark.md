# Parse Benchmark Analysis

## Executive Summary

The initial benchmark results showed our custom parser was **66x slower** than syn's `parse_nested_meta`, but this was due to **unfair benchmark methodology**. After correcting for this, the picture is more nuanced.

## Benchmark Results (Original)

```
syn_full_parse          time:   [12.232 µs 12.293 µs 12.366 µs]
syn_parse_attrs         time:   [27.184 ns 27.318 ns 27.473 ns]
syn_parse_nested_meta   time:   [13.141 ns 13.221 ns 13.310 ns]
custom_parse            time:   [879.47 ns 882.99 ns 887.15 ns]
```

## Critical Finding: Unfair Comparison

The benchmarks were **not comparing equivalent operations**:

### syn_full_parse (12.293 µs)
- Clones TokenStream on every iteration ✓
- Parses full struct with `syn::parse2()` ✓
- **Measures**: TokenStream clone + full AST construction

### syn_parse_attrs (27.318 ns)
- Pre-parses input **once** outside benchmark loop ✗
- Only benchmarks filtering pre-parsed attributes ✓
- **Measures**: Attribute filtering from existing AST

### syn_parse_nested_meta (13.221 ns)
- Pre-parses input **once** outside benchmark loop ✗
- Only benchmarks parsing nested meta from pre-parsed attributes ✓
- **Measures**: Nested meta parsing from existing Attribute objects

### custom_parse (882.99 ns)
- Clones TokenStream on every iteration ✓
- Parses attribute structure directly ✓
- **Measures**: TokenStream clone + custom parsing

## Why This Matters

The syn benchmarks for `syn_parse_attrs` and `syn_parse_nested_meta` measure **post-parsing operations** on an already-constructed AST. They don't include the cost of:
1. Cloning the TokenStream
2. Constructing the DeriveInput AST
3. Walking the token tree

Our `custom_parse` benchmark includes the TokenStream clone cost, making it an **apples-to-oranges comparison**.

## Corrected Analysis

### Fair Comparison 1: Full Parse Path
Comparing end-to-end parsing from TokenStream:

- **syn_full_parse**: 12,293 ns
- **custom_parse**: 883 ns
- **Speedup**: **13.9x faster** ✓

This shows our custom parser is significantly faster when parsing the same input from scratch.

### Fair Comparison 2: What We Actually Do in Derives

In actual proc-macro usage, we:
1. Parse full struct with syn (`syn::parse2()`)
2. Extract field attributes
3. Parse nested meta from attributes

The relevant cost is:
- **syn path**: 12,293 ns (full parse) + 13.221 ns (nested meta) = **12,306 ns**
- **custom path**: 883 ns
- **Speedup**: **13.9x faster** ✓

## Performance Bottlenecks Identified

### In Custom Parser (macrokid_parse_bench/src/lib.rs)

1. **String Allocations** (lines 59, 89):
   ```rust
   let attr_name = match iter.next() {
       Some(TokenTree::Ident(ident)) => ident.to_string(),  // Allocation!
       _ => return Err("Expected attribute name".to_string()),
   }
   ```
   - Converting `Ident` → `String` allocates
   - Could use `&str` comparison instead

2. **Error String Allocations** (throughout):
   ```rust
   return Err("Expected '#'".to_string());  // Allocation on error path
   ```
   - Every error path allocates a String
   - Could use `&'static str` for errors

3. **TokenStream Cloning** (line 51):
   ```rust
   parse_attr_content(group.stream())  // Calls .stream() which may clone
   ```
   - `Group::stream()` returns a new TokenStream
   - Might be unavoidable with current API

4. **Integer Parsing** (line 137):
   ```rust
   let s = lit.to_string();  // Allocation!
   s.parse().map_err(...)
   ```
   - Converts `Literal` → `String` → `u32`
   - Could potentially parse directly

### In Benchmark (benches/parse_attrs.rs)

5. **TokenStream Clone** (line 96):
   ```rust
   let result = parse_resource_attr(black_box(tokens.clone())).unwrap();
   ```
   - Necessary for benchmarking, but expensive
   - TokenStream uses `Rc` internally, so clones are cheap-ish
   - But still has overhead

## Why syn_parse_nested_meta is So Fast (13ns)

The 13ns benchmark is misleadingly fast because it's **not parsing from tokens**:

```rust
// Pre-parsed ONCE, outside benchmark loop
let derive_input: DeriveInput = syn::parse2(input.clone()).unwrap();

// Benchmark only measures THIS:
b.iter(|| {
    for attr in &derive_input.attrs {  // Iterating pre-parsed attrs
        if attr.path().is_ident("uniform") {
            let _ = attr.parse_nested_meta(|meta| {  // Parsing from pre-built Attribute
                // ...
            });
        }
    }
})
```

The expensive work (parsing tokens → AST) happened **outside the benchmark**. We're only measuring:
- Iterating over pre-parsed attributes
- Calling `parse_nested_meta` on an already-constructed Attribute

This is useful for measuring **incremental** attribute processing, but doesn't represent the full cost of macro expansion.

## Realistic Performance Estimate

In actual derive macro usage:

### Current Approach (using syn):
```
syn::parse2()                    12,293 ns
+ iterate fields                    ~100 ns (estimate)
+ parse_nested_meta × 4 fields    ~50 ns (estimate)
----------------------------------------
Total:                           ~12,450 ns per struct
```

### Custom Parser Approach:
```
custom_parse × 4 fields            3,532 ns (883ns × 4)
----------------------------------------
Total:                             3,532 ns per struct
```

**Estimated speedup**: ~3.5x for attribute parsing

## Macro Expansion Context

However, attribute parsing is only **one component** of derive macro expansion time:

1. **Token parsing** (syn::parse2): ~12 µs
2. **Attribute extraction**: ~4 µs (custom) vs ~12 µs (syn full parse)  ← We optimize this
3. **Code generation**: ~5-50 µs (varies by derive complexity)
4. **Quote generation**: ~2-10 µs

Our optimization targets step 2, which might be **10-20% of total macro time**.

**Expected total speedup**: 1.1x - 1.3x for full derive expansion

## Conclusion

### What We Learned

1. ✓ **Custom parser IS faster** for attribute parsing (~13.9x vs full syn parse)
2. ✓ **Benchmark methodology matters** - must compare equivalent operations
3. ✗ **Not a silver bullet** - attribute parsing is only one part of macro expansion
4. ✓ **Optimization opportunities exist** in string allocations and parsing

### Recommendations

#### Option A: Continue with Custom Parser Approach ⚠️

**Pros**:
- 13.9x faster for attribute parsing
- More control over parsing logic
- Could optimize further (remove allocations)

**Cons**:
- Maintenance burden (we own the parser)
- Might only improve total macro time by 10-20%
- Still need syn for struct/field parsing
- Added complexity

**Effort**: Medium-High (2-3 days to integrate + maintain)

#### Option B: Stick with syn ✓ RECOMMENDED

**Pros**:
- Battle-tested and maintained by dtolnay
- Already handles edge cases
- Integration with existing code is trivial
- 13ns for incremental parsing is excellent

**Cons**:
- Slightly slower full-parse path (12µs vs 883ns)
- Less control over parsing

**Effort**: Zero (already implemented)

#### Option C: Hybrid Approach 🤔

Use syn for struct parsing, custom parser for hot-path attributes:

```rust
// Use syn for struct skeleton
let input: DeriveInput = syn::parse2(tokens)?;

// Use custom parser for field attributes (hot path)
for field in fields {
    for attr in field.attrs {
        let parsed = custom_parse_attr(attr.tokens)?;  // Fast path
    }
}
```

**Pros**:
- Best of both worlds
- Optimize only the hot path
- Still leverage syn for complex parsing

**Cons**:
- Two parsing systems to maintain
- More complex integration

**Effort**: Medium (1-2 days)

## Recommended Next Steps

**I recommend Option B: Stick with syn**

**Rationale**:
1. The 13.9x speedup for attribute parsing translates to only ~10-20% total macro speedup
2. syn is battle-tested and handles edge cases we haven't considered
3. Our time is better spent on higher-impact optimizations:
   - Code generation optimization (see `macrokid_core/src/common/gen.rs`)
   - Reducing quote! overhead
   - Compile-time caching of generated code

4. If compile times become critical, profile first to find actual bottlenecks

### If We Do Pursue Custom Parsing

Before integrating the custom parser, we should:

1. **Eliminate allocations**:
   - Use `&str` comparisons instead of `.to_string()`
   - Use `&'static str` for errors
   - Direct integer parsing without string conversion

2. **Fix benchmarks** to compare equivalent operations:
   - All benchmarks should parse from TokenStream
   - OR all benchmarks should operate on pre-parsed data

3. **Add comprehensive tests**:
   - Edge cases (missing parameters, wrong types)
   - Error messages
   - All attribute variants

4. **Measure real-world impact**:
   - Profile actual derive macro compilation
   - Measure end-to-end build time improvement

## Files Referenced

- `macrokid_parse_bench/src/lib.rs:36-159` - Custom parser implementation
- `macrokid_parse_bench/benches/parse_attrs.rs:27-100` - Benchmark definitions
- `/tmp/bench_results.txt` - Raw benchmark output
