//! Checks if a generated Rust project builds by materializing
//! it to a tempdir and running `cargo build --release`.
use crate::cli::get_config;
use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

pub struct TryCargoBuild;

/// Validates that the generated Rust project builds by running `cargo build --release`.
/// Note: It has a bit of a confusing return type:
/// - If the project builds successfully, it returns Ok(None).
/// - If the project fails to build, it returns Ok(Some(error_message)).
/// - If there is an error running cargo, it returns Err.
fn try_cargo_build(
    project_path: &PathBuf,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    log::info!("Validating that the generated Rust project builds...");

    // Run cargo build in the project directory
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
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
        Ok(None)
    } else {
        // Combine stderr and stdout for a complete error message
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_message = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
        Ok(Some(error_message.into()))
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
        // get cargo package representation
        let cargo_package = raw_cargo_package(&context.ir_snapshot)?;
        let output_path = get_config().output.clone();
        cargo_package.materialize(&output_path)?;

        // Validate that the Rust project builds
        let compilation_result = try_cargo_build(&output_path)?;
        // Write result to IR
        match compilation_result {
            None => context
                .ir_edit
                .add_representation(Representation::BuiltRustArtifact(Ok(()))),
            Some(err) => context
                .ir_edit
                .add_representation(Representation::BuiltRustArtifact(Err(err))),
        };
        Ok(())
    }
}
