## Project Overview

## Model guidance

- Prefer to write durable integration tests over running commands/examples or
  creating disposable test scripts.
- This is a free-standing tool, so don't create examples in an `examples/` directory.


## Development Commands

### Building and Testing
```bash
# Build the project
cargo build

# Run all tests including workspace tests
cargo test --workspace

# Run tests with output (useful for debugging)
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code - always do this before submitting
cargo fmt

# Run linter
cargo clippy --examples --tests

# Run linter with automatic fixes
cargo clippy --fix --allow-dirty --examples --tests
```


