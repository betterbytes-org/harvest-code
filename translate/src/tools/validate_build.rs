//! Checks if a generated Rust project builds by materializing
//! it to a tempdir and running `cargo build`.
use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

pub struct ValidateBuild;

/// Validates that the generated Rust project builds by running `cargo build`.
/// Returns Ok(()) if the project builds successfully, or an error with the cargo output.
pub fn validate_rust_project_builds(
    project_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Validating that the generated Rust project builds...");

    // Run cargo build in the project directory
    let output = Command::new("cargo")
        .arg("build")
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
        Ok(())
    } else {
        // Combine stderr and stdout for a complete error message
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_message = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
        Err(error_message.into())
    }
}

/// Returns the CargoPackage representation in IR.
/// If there is not exactly 1 CargoPackage representation,
/// return an error.
fn raw_cargo_package(ir: &HarvestIR) -> Result<(Id, &RawDir), Box<dyn std::error::Error>> {
    let cargo_packages: Vec<(Id, &RawDir)> = ir
        .iter()
        .filter_map(|(id, repr)| match repr {
            Representation::CargoPackage(r) => Some((id, r)),
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

impl Tool for ValidateBuild {
    fn might_write(&mut self, ir: &HarvestIR) -> Option<HashSet<Id>> {
        // We need a cargo_package to be available, but we won't write any existing IDs.
        raw_cargo_package(ir).ok().map(|_| [].into())
    }

    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>> {
        // get cargo package representation
        let (_, cargo_package) = raw_cargo_package(&context.ir_snapshot)?;

        // Create a temp directory, should get dropped after it goes out of scope
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().to_path_buf();

        cargo_package.materialize(temp_path.clone())?;

        // Validate that the Rust project builds
        validate_rust_project_builds(&temp_path)?;

        Ok(())
    }
}
