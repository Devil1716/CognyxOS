# Coding standards

- Python: Python 3.13, strict typed public APIs, Ruff formatting and linting, `snake_case` functions and modules, `PascalCase` types.
- TypeScript: strict mode, ESLint and Prettier, `camelCase` values, `PascalCase` types, package names prefixed `@cognyx/`.
- Rust: stable edition 2024, `cargo fmt`, `cargo clippy -D warnings`.
- Logging: structured JSON only, with event names in `snake_case`; do not log secrets or sensitive user data.
- Errors: derive from the shared Cognyx error hierarchy; do not throw generic errors across module boundaries.

Use `configs/base.yaml` for defaults and environment variables only for deployment-specific overrides.
