//! Attempts to directly turn a C project into a Cargo project by throwing it at
//! an LLM via the `llm` crate.

use crate::cli::{get_config, unknown_field_warning};
use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::{ChatMessage, StructuredOutputFormat};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf, str::FromStr};

/// Structured output JSON schema for Ollama.
const STRUCTURED_OUTPUT_SCHEMA: &str =
    include_str!("raw_source_to_cargo_llm/structured_schema.json");

const SYSTEM_PROMPT: &str = include_str!("raw_source_to_cargo_llm/system_prompt.txt");

pub struct RawSourceToCargoLlm;

impl Tool for RawSourceToCargoLlm {
    fn might_write(&mut self, ir: &HarvestIR) -> Option<Vec<Id>> {
        // We need a raw_source to be available, but we won't write any existing IDs.
        raw_source(ir).map(|_| vec![])
    }

    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>> {
        let config = get_config();
        let config = &config.tools.raw_source_to_cargo_llm;
        let in_dir = raw_source(&context.ir_snapshot).unwrap();

        // Use the llm crate to connect to Ollama.

        let output_format: StructuredOutputFormat = serde_json::from_str(STRUCTURED_OUTPUT_SCHEMA)?;
        let llm = LLMBuilder::new()
            .backend(LLMBackend::from_str(&config.backend).expect("unknown LLM_BACKEND"))
            .base_url(format!("http://{}", config.address))
            .model(&config.model)
            .max_tokens(100000)
            .temperature(0.0) // Suggestion from https://ollama.com/blog/structured-outputs
            .schema(output_format)
            .system(SYSTEM_PROMPT)
            .build()
            .expect("Failed to build LLM (Ollama)");

        // Assemble the Ollama request.
        let mut request = vec!["Please translate the following C project into a Rust project including Cargo manifest:".into()];
        for (path, contents) in in_dir.files_recursive() {
            request.push(format!(
                "{} contains:\n{}",
                path.to_string_lossy(),
                String::from_utf8_lossy(contents)
            ));
        }
        // "return as JSON" is suggested by https://ollama.com/blog/structured-outputs
        request.push("return as JSON".into());
        let request: Vec<_> = request
            .iter()
            .map(|contents| ChatMessage::user().content(contents).build())
            .collect();

        // Make the Ollama call.
        let response = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("tokio failed")
            .block_on(llm.chat(&request))?
            .text()
            .expect("no response text");

        // Parse the response, convert it into a CargoPackage representation.
        let files: Vec<OutputFile> = serde_json::from_str(&response)?;
        let mut out_dir = RawDir::default();
        for file in files {
            out_dir.set_file(&file.path, file.contents.into())?;
        }
        context
            .ir_edit
            .add_representation(Representation::CargoPackage(out_dir));
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Hostname and port at which to find the LLM serve. Example: [::1]:11434
    address: String,

    /// Which backend to use, e.g. "ollama".
    backend: String,

    /// Name of the model to invoke.
    model: String,

    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}

impl Config {
    pub fn validate(&self) {
        unknown_field_warning("tools.raw_source_to_cargo_llm", &self.unknown);
    }
}

/// Returns the RawSource representation in IR. If there are multiple RawSource representations,
/// returns an arbitrary one.
fn raw_source(ir: &HarvestIR) -> Option<&RawDir> {
    ir.iter().find_map(|(_, repr)| match repr {
        Representation::RawSource(r) => Some(r),
        _ => None,
    })
}

/// Structure representing a file created by the LLM.
#[derive(Debug, Deserialize)]
struct OutputFile {
    contents: String,
    path: PathBuf,
}
