# HARVEST code

A place to put HARVEST code that has not yet been migrated into its own
repository.

## Building the Rust code

If you have [rustup](https://rustup.rs) installed, you can build the code by
running:

```bash
cargo build --release
```

If you do not use rustup, you will need a sufficiently-new stable Rust compiler
(see rust-toolchain.toml for a toolchain version that is known to work).

## LLM server

You will also need an LLM server. This can be local, or remote. A couple options
are given below:

### Local Ollama instance

You can follow Ollama's [download instructions](https://ollama.com/download), or
download its [Docker image](https://hub.docker.com/r/ollama/ollama).

Once you have it installed, you need to download a model. By default,
harvest_translate uses `codellama:7b`:

```bash
ollama pull codellama:7b                       # If installed in your system
docker container run ollama pull codellama:7b  # If using Docker
```

You will need to have Ollama running to run harvest_translate.

## Running

### Translate C code to Rust
```bash
cargo run --release -- /path/to/c/code -o /path/to/output
```

### Configuration
Print config file location:
```bash
cargo run -- --print-config-path
```

You can find more information on configuration in [docs/Configuration.md].
