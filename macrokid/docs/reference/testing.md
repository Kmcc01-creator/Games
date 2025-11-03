# Testing Guide

This project uses standard Cargo tests. Some experimental examples may not compile on all systems, so we recommend running package-scoped tests during development.

## Quick Commands

- Core tests only (recommended):
  - `cargo test -p macrokid_core`
  - With DSL feature: `cargo test -p macrokid_core --features pattern_dsl`

- Build entire workspace (may include experimental examples):
  - `cargo build`
  - Run gfx DSL demo: `cargo run -p gfx_dsl --example demo`
  - Run resource derives demo: `cargo run -p render_resources --example demo`

## Notes

- Unit tests currently cover attribute parsing helpers (Phase 2) in `macrokid_core`.
- Feature-gated tests for the typed pattern DSL (`pattern_dsl`) validate struct patterns with and without `..`, tuple patterns, `or`, and guards.
- Example-crate tests:
  - `render_resources`: validations for duplicate bindings/locations.
  - `gfx_dsl_support`: builder tests.

### Feature flags

- Enable `pattern_dsl` and run tests:
  - `cargo test -p macrokid_core --features pattern_dsl`
- More tests will be added as new phases land (trybuild UI tests, expanded examples).
- If you encounter build errors from experimental example crates, focus on package-scoped testing as above.

## Future (Planned)

- Add `trybuild` UI tests for error diagnostics (requires adding dev-dependency).
- Add snapshot tests for token-stream output of builders.
- CI workflow to run `cargo fmt --check`, `cargo clippy -D warnings`, and targeted `cargo test`.
