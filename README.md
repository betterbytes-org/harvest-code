# HARVEST code

A place to put HARVEST code that has not yet been migrated into its own
repository.

## Usage

### Translate C code to Rust
```bash
cargo run --bin=harvest_translate -- /path/to/c/code -o /path/to/output
```

### Configuration
Print config file location:
```bash
cargo run --bin=harvest_translate -- --print-config-path
```


