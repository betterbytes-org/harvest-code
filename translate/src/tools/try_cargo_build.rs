//! Checks if a generated Rust project builds by materializing
//! it to a tempdir and running `cargo build --release`.
use crate::cli::unknown_field_warning;
use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct TryCargoBuild;
// Either a vector of compiled artifact filenames (on success)
// or a string containing error messages (on failure).
pub type BuildResult = Result<Vec<PathBuf>, String>;

/// Parses cargo output stream and concatenates all compiler messages into a single string.
fn parse_compiler_messages(stdout: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let mut messages = Vec::new();

    for message in cargo_metadata::Message::parse_stream(stdout) {
        let message = message?;
        if let cargo_metadata::Message::CompilerMessage(comp_msg) = message {
            messages.push(format!("Compiler Message: {}", comp_msg));
        }
    }

    Ok(messages.join("\n"))
}

/// Parses cargo output stream and extracts the filenames of all compiled artifacts.
/// Returns a vector of PathBuf containing the artifact filenames.
fn parse_compiled_artifacts(stdout: &[u8]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut artifact_filenames = Vec::new();

    for message in cargo_metadata::Message::parse_stream(stdout) {
        let message = message?;
        if let cargo_metadata::Message::CompilerArtifact(artifact) = message {
            // Extract filenames from all artifact files
            for filename in artifact.filenames {
                artifact_filenames.push(filename.into());
            }
        }
    }

    Ok(artifact_filenames)
}

/// Validates that the generated Rust project builds by running `cargo build --release`.
/// Note: It has a bit of a confusing return type:
/// - If the project builds successfully, it returns Ok(Ok(artifact_filenames)).
/// - If the project fails to build, it returns Ok(Err(error_message)).
/// - If there is an error running cargo, it returns Err.
fn try_cargo_build(project_path: &PathBuf) -> Result<BuildResult, Box<dyn std::error::Error>> {
    log::info!("Validating that the generated Rust project builds...");

    // Run cargo build in the project directory
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--message-format=json")
        .current_dir(project_path)
        .output()
        .map_err(|e| {
            format!(
                "Failed to run cargo build in {}: {}",
                project_path.display(),
                e
            )
        })?;

    if output.status.success() {
        log::info!("Project builds successfully!");
        let artifact_filenames = parse_compiled_artifacts(&output.stdout)?;
        Ok(Ok(artifact_filenames))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let compiler_messages = parse_compiler_messages(&output.stdout)?;
        let error_message = format!("{}\n{}", compiler_messages, stderr);
        Ok(Err(error_message))
    }
}
/// Returns the CargoPackage representation in IR.
/// If there is not exactly 1 CargoPackage representation,
/// return an error.
fn raw_cargo_package(ir: &HarvestIR) -> Result<&RawDir, Box<dyn std::error::Error>> {
    let cargo_packages: Vec<&RawDir> = ir
        .iter()
        .filter_map(|(_, repr)| match repr {
            Representation::CargoPackage(r) => Some(r),
            _ => None,
        })
        .collect();

    match cargo_packages.len() {
        0 => Err("No CargoPackage representation found in IR".into()),
        1 => Ok(cargo_packages[0]),
        n => Err(format!(
            "Found {} CargoPackage representations, expected at most 1",
            n
        )
        .into()),
    }
}

impl Tool for TryCargoBuild {
    fn might_write(&mut self, ir: &HarvestIR) -> Option<HashSet<Id>> {
        // We need a cargo_package to be available, but we won't write any existing IDs.
        raw_cargo_package(ir).ok().map(|_| [].into())
    }

    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>> {
        // Get cargo package representation
        let cargo_package = raw_cargo_package(&context.ir_snapshot)?;
        let output_path = context.config.output.clone();
        cargo_package.materialize(&output_path)?;

        // Validate that the Rust project builds
        let compilation_result = try_cargo_build(&output_path)?;
        // Write result to IR
        context
            .ir_edit
            .add_representation(Representation::CargoBuildResult(compilation_result));

        Ok(())
    }
}
