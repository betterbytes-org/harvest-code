use clap::{Arg, Parser};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "harvest-benchmark")]
#[command(
    about = "Runs all benchmarks by translating C projects to Rust and validating them with test vectors"
)]
pub struct Args {
    /// Input directory containing subdirectories with benchmarks
    #[arg(
        help = "Path to the directory containing example subdirectories (each with test_case/ and test_vectors/)"
    )]
    pub input_dir: PathBuf,

    /// Output directory where the translated Rust projects will be written
    #[arg(help = "Path to the output directory for all translated Rust projects")]
    pub output_dir: PathBuf,

    /// Set a configuration value; format $NAME=$VALUE.
    #[arg(long, short)]
    pub config: Vec<String>,
}

// /// Configuration for this harvest-benchmark run. The sources of these configuration values (from
// /// highest-precedence to lowest-precedence) are:
// ///
// /// 1. Configurations passed using the `--config` command line flag.
// /// 2. A user-specific configuration directory (e.g. `$HOME/.config/harvest/config.toml').
// /// 3. Defaults specified in the code (using `#[serde(default)]`).
// #[derive(Debug, Deserialize)]
// pub struct Config {
//     /// Input directory containing subdirectories with MITLL examples
//     pub in_performer: PathBuf,

//     /// Path to output directory.
//     pub output: PathBuf,

//     // serde will place any unrecognized fields here. This will be passed to unknown_field_warning
//     // after parsing to emit warnings on unrecognized config entries (we don't error on unknown
//     // fields because that can be annoying to work with if you are switching back and forth between
//     // commits that have different config options).
//     #[serde(flatten)]
//     unknown: HashMap<String, Value>,
// }
