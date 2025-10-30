use crate::tools;
use clap::Parser;
use config::FileFormat::Toml;
use directories::ProjectDirs;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Parser)]
pub struct Args {
    /// Set a configuration value; format $NAME=$VALUE.
    #[arg(long, short)]
    pub config: Vec<String>,

    /// Path to the directory containing the C code to translate.
    // Should always be present unless using a subcommand like --print-config-path
    pub input: Option<PathBuf>,

    /// Prints out the location of the config file.
    #[arg(long)]
    pub print_config_path: bool,

    /// Path to output directory containing the translated Rust code.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Configuration for this harvest-translate run. The sources of these configuration values (from
/// highest-precedence to lowest-precedence) are:
///
/// 1. Configurations passed using the `--config` command line flag.
/// 2. A user-specific configuration directory (e.g. `$HOME/.config/harvest/config.toml').
/// 3. Defaults specified in the code (using `#[serde(default)]`).
#[derive(Debug, Deserialize)]
pub struct Config {
    ///  Path to the directory containing the C code to translate.
    pub input: PathBuf,

    /// Path to output directory.
    pub output: PathBuf,

    /// Sub-configuration for each tool.
    pub tools: tools::ToolConfigs,

    // serde will place any unrecognized fields here. This will be passed to unknown_field_warning
    // after parsing to emit warnings on unrecognized config entries (we don't error on unknown
    // fields because that can be annoying to work with if you are switching back and forth between
    // commits that have different config options).
    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}

/// Prints out a warning message for every field in `unknown`.
///
/// This is intended for use by config validation routines. `prefix` should be the path to this
/// entry (e.g. `tools::Config` should call this with a `prefix` of `tools`).
pub fn unknown_field_warning(prefix: &str, unknown: &HashMap<String, Value>) {
    let mut entries: Vec<_> = unknown.keys().collect();
    entries.sort_unstable();
    entries.into_iter().for_each(|name| match prefix {
        "" => eprintln!("Warning: unknown config key {name}"),
        p => eprintln!("Warning: unknown config key {p}.{name}"),
    });
}

/// Performs parsing and validation of the config; to be called by main() before executing any code
/// that tries to retrieve the config.
///
/// Returns the config, or None if a command line flag that calls for an early exit (such as --print_config_path)
/// was provided.
pub fn initialize() -> Option<Arc<Config>> {
    let args: Arc<_> = Args::parse().into();
    let dirs = ProjectDirs::from("", "", "harvest").expect("no home directory");
    if args.print_config_path {
        println!("Config file location: {:?}", config_file(dirs.config_dir()));
        return None;
    }
    let config = load_config(&args, dirs.config_dir());
    unknown_field_warning("", &config.unknown);
    config.tools.validate();
    Some(config.into())
}

fn load_config(args: &Args, config_dir: &Path) -> Config {
    let mut settings = config::Config::builder()
        .add_source(config::File::from_str(
            include_str!("../default_config.toml"),
            Toml,
        ))
        .add_source(config::File::from(config_file(config_dir)).required(false))
        .add_source(config::File::from(PathBuf::from("config.toml")).required(false));
    for config_arg in &args.config {
        let Some((name, value)) = config_arg.split_once('=') else {
            panic!("failed to parse config value {config_arg:?}; no '=' found");
        };
        settings = settings
            .set_override(name, value)
            .expect("settings override failed");
    }

    // We need to set an override so that deserializing the config does not error.
    // However, the config crate does not support providing a Path in an override.
    // We could convert to a string and back, but that can be lossy. Instead, this just sets a
    // blank value and then corrects it after deserialization.
    if args.input.is_some() {
        settings = settings
            .set_override("input", " ")
            .expect("settings override failed");
    }

    if args.output.is_some() {
        settings = settings
            .set_override("output", " ")
            .expect("settings override failed");
    }

    let mut config: Config = settings
        .build()
        .expect("failed to build settings")
        .try_deserialize()
        .expect("config deserialization failed");
    if let Some(ref input) = args.input {
        config.input = input.clone();
    }
    if let Some(ref output) = args.output {
        config.output = output.clone();
    }
    config
}

/// Returns the config file path, given the config directory.
fn config_file(config_dir: &Path) -> PathBuf {
    [config_dir, "translate.toml".as_ref()].iter().collect()
}

#[cfg(test)]
mod tests {
    #[cfg(not(miri))]
    #[test]
    fn load_config_test() {
        use super::*;
        use crate::test_util::tempdir;
        use std::{fs, io::Write as _};
        let config_dir = tempdir().unwrap();

        assert_eq!(
            load_config(
                &Args::parse_from(["", "a", "--output=/tmp/out"]),
                config_dir.path(),
            )
            .input,
            AsRef::<Path>::as_ref("a")
        );

        fs::File::create(config_file(config_dir.path()))
            .unwrap()
            .write(
                br#"
                    input = "b"
                    [tools.raw_source_to_cargo_llm]
                    address = "127.0.0.1"
                    model = "gpt-oss"
                "#,
            )
            .unwrap();
        assert_eq!(
            load_config(
                &Args::parse_from(["", "--output=/tmp/out"]),
                config_dir.path()
            )
            .input,
            AsRef::<Path>::as_ref("b")
        );
        // Verify the --config flag overrides the user's config file.
        assert_eq!(
            load_config(
                &Args::parse_from(["", "--config", "input=c", "--output=/tmp/out"]),
                config_dir.path()
            )
            .input,
            AsRef::<Path>::as_ref("c")
        );
        // Verify --input overrides all the configuration options.
        assert_eq!(
            load_config(
                &Args::parse_from(["", "--config", "input=d", "d", "--output=/tmp/out"]),
                config_dir.path()
            )
            .input,
            AsRef::<Path>::as_ref("d")
        );
    }
}
