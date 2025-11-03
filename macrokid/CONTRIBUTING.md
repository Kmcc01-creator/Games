# Contributing to Macrokid

Thank you for your interest in contributing to Macrokid! This document provides guidelines and information for contributors.

## Code of Conduct

- Be respectful and constructive
- Focus on what's best for the project
- Welcome newcomers and help them learn

## Getting Started

1. **Read the Documentation**
   - Start with [README.md](README.md) for project overview
   - Review [docs/guides/getting-started.md](docs/guides/getting-started.md)
   - Check [docs/architecture/overview.md](docs/architecture/overview.md) for design principles

2. **Set Up Development Environment**
   - Rust 1.70+ (2021 edition)
   - Clone the repository
   - Run `cargo test` to verify setup
   - Try building examples

3. **Choose a Contribution Area**
   - Check [TODO.md](TODO.md) for current priorities
   - Look for "good first issue" labels (if using issue tracker)
   - Ask in discussions if unsure where to start

## Types of Contributions

### 1. Bug Fixes

1. Search existing issues to avoid duplicates
2. Create a new issue describing the bug
3. Fork and create a branch: `fix/issue-description`
4. Write tests that reproduce the bug
5. Fix the bug and verify tests pass
6. Submit a pull request

### 2. New Features

1. Open an issue to discuss the feature first
2. Get feedback on design approach
3. Fork and create a branch: `feature/feature-name`
4. Implement the feature
5. Add tests and documentation
6. Submit a pull request

### 3. Documentation

Documentation improvements are always welcome:
- Fix typos or unclear explanations
- Add examples
- Improve getting started guides
- Document common patterns

Location guidelines:
- Cross-cutting docs: `docs/`
- Library-specific docs: `<crate>/docs/`
- Always update relevant README files

### 4. Code Quality

- Refactoring for clarity
- Performance improvements
- Test coverage improvements
- Better error messages

## Development Workflow

### Building

```bash
# Build all crates
cargo build

# Build specific crate
cargo build -p macrokid_core

# Build with features
cargo build -p macrokid_graphics --features vulkan-linux
```

### Testing

```bash
# Run all tests
cargo test

# Test specific crate
cargo test -p macrokid_core

# Run benchmarks
cargo bench -p macrokid_parse_bench
```

### Examples

```bash
# Run graphics examples
cargo run -p macrokid_graphics --example linux_vulkan \
    --features vulkan-linux,vk-shaderc-compile

# Run custom derive examples
cargo run --bin example
```

## Code Style

### Rust Style

- Follow standard Rust conventions (rustfmt)
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings

### Documentation

- Add doc comments to public APIs
- Include examples in doc comments
- Use clear, concise language
- Update relevant guides when adding features

### Commit Messages

Use clear, descriptive commit messages:

```
Add float support to AttrSchema

- Added req_float() and opt_float() methods
- Added get_float() to ParsedAttrs
- Updated exclusive_schemas! macro
- Added tests for float parsing
```

Format:
- First line: Brief summary (50 chars or less)
- Blank line
- Detailed description (72 chars per line)
- List specific changes with bullet points

## Pull Request Process

1. **Before Submitting**
   - Ensure all tests pass
   - Run `cargo fmt` and `cargo clippy`
   - Update documentation
   - Add CHANGELOG.md entry if applicable

2. **PR Description**
   - Describe the problem being solved
   - Explain your approach
   - List any breaking changes
   - Link related issues

3. **Review Process**
   - Address reviewer feedback
   - Keep commits clean (squash if needed)
   - Be responsive to questions

4. **After Merge**
   - Delete your branch
   - Update your fork

## Project Structure

```
macrokid/
├── macrokid/               # Main proc-macro crate
├── macrokid_core/          # Framework core
│   ├── src/
│   │   ├── ir.rs           # Type introspection
│   │   ├── common/         # Shared utilities
│   │   └── ...
│   ├── docs/               # Core-specific docs
│   └── README.md
├── macrokid_graphics/      # Graphics runtime
│   ├── src/
│   ├── docs/               # Graphics docs
│   ├── examples/
│   └── README.md
├── macrokid_graphics_derive/ # Graphics derives
├── macrokid_threads_derive/  # Threading derives
├── docs/                   # Project-wide docs
│   ├── architecture/
│   ├── guides/
│   ├── reference/
│   └── design/
├── CHANGELOG.md
├── CONTRIBUTING.md         # This file
└── README.md
```

## Common Contribution Areas

### 1. New Builder Patterns

Add helpers for common code generation patterns:
- Location: `macrokid_core/src/common/builders.rs`
- Extend `ImplBuilder` or create new builders
- Add tests and examples

### 2. Attribute Parsers

Extend attribute parsing:
- Location: `macrokid_core/src/common/attrs.rs`
- Add new attribute types or validators
- Update schema system in `attr_schema.rs`

### 3. Example Macros

Demonstrate framework usage:
- Location: `examples/`
- Show real-world use cases
- Document patterns and best practices

### 4. Graphics Derives

Add new graphics-related derives:
- Location: `macrokid_graphics_derive/src/`
- Follow existing patterns (ResourceBinding, BufferLayout)
- Test with Vulkan backend

### 5. Performance

Optimize compilation or runtime:
- Profile first (use `macrokid_parse_bench`)
- Document findings
- Measure improvements

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

### Integration Tests

Place in `tests/` directory at crate root.

### Benchmark Tests

Add to `macrokid_parse_bench/benches/` for performance-sensitive code.

## Documentation Guidelines

### Code Comments

```rust
/// Brief description of function
///
/// # Arguments
/// * `param` - Description of parameter
///
/// # Returns
/// Description of return value
///
/// # Examples
/// ```
/// let result = my_function(42);
/// assert_eq!(result, expected);
/// ```
pub fn my_function(param: i32) -> String {
    // Implementation
}
```

### Markdown Documentation

- Use clear headings
- Include code examples
- Link to related documentation
- Keep examples up to date

## Questions?

- Check existing documentation
- Look through closed issues
- Ask in discussions (if available)
- Reach out to maintainers

## Recognition

Contributors will be recognized in:
- Git commit history
- Release notes
- Project acknowledgments

Thank you for contributing to Macrokid!
