# Fetch Taxonomy

This example script demonstrates how the React Web Application should dynamically build its Error Dictionary by parsing the core Rust engine's source of truth TOML files.

Currently, the React frontend hardcodes Soroban error numbers (Issue #4), which is unmaintainable. This script reads `crates/core/src/taxonomy/data/contract.toml`, parses it using the `@iarna/toml` library, and writes a normalized JSON dictionary.

## Installation

```bash
pnpm install
# or npm install
```

## Running the Example

```bash
npm start
```
This will read the TOML file from the `crates/core` directory and output a JSON format to the console or write it to a `taxonomy.json` file.
