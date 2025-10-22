//! Attempts to directly turn a C project into a Cargo project by throwing it at
//! an LLM via the `llm` crate.

use crate::cli::{get_config, unknown_field_warning};
use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::{ChatMessage, StructuredOutputFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;

use super::ToolInvocation;

/// Structured output JSON schema for Ollama.
const STRUCTURED_OUTPUT_SCHEMA: &str =
    include_str!("raw_source_to_cargo_llm/structured_schema.json");

const SYSTEM_PROMPT: &str = include_str!("raw_source_to_cargo_llm/system_prompt.txt");

pub struct Invocation;

impl ToolInvocation for Invocation {
    fn create_tool(&self) -> Box<dyn Tool> {
        Box::new(RawSourceToCargoLlm)
    }
}

impl std::fmt::Display for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RawSourceToCargoLlm")
    }
}

pub struct RawSourceToCargoLlm;

impl Tool for RawSourceToCargoLlm {
    fn might_write(&mut self, ir: &HarvestIR) -> Option<HashSet<Id>> {
        // We need a raw_source to be available, but we won't write any existing IDs.
        raw_source(ir).map(|_| [].into())
    }

    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>> {
        let config = &get_config().tools.raw_source_to_cargo_llm;
        log::debug!("LLM Configuration {config:?}");
        let in_dir = raw_source(&context.ir_snapshot).unwrap();

        // Use the llm crate to connect to Ollama.

        let output_format: StructuredOutputFormat = serde_json::from_str(STRUCTURED_OUTPUT_SCHEMA)?;
        let llm = {
            let mut llm_builder = LLMBuilder::new()
                .backend(LLMBackend::from_str(&config.backend).expect("unknown LLM_BACKEND"))
                .model(&config.model)
                .max_tokens(config.max_tokens)
                .temperature(0.0) // Suggestion from https://ollama.com/blog/structured-outputs
                .schema(output_format)
                .system(SYSTEM_PROMPT);

            if let Some(ref address) = config.address
                && !address.is_empty()
            {
                llm_builder = llm_builder.base_url(address);
            }
            if let Some(ref api_key) = config.api_key
                && !api_key.0.is_empty()
            {
                llm_builder = llm_builder.api_key(&api_key.0);
            }

            llm_builder.build().expect("Failed to build LLM (Ollama)")
        };

        // Assemble the Ollama request.
        let mut request = vec!["Please translate the following C project into a Rust project including Cargo manifest:".into()];
        request.push(
            serde_json::json!({"files": (&in_dir.files_recursive().iter().map(|(path, contents)| {
                OutputFile {
                    path: path.clone(),
                    contents: String::from_utf8_lossy(contents).into(),
                }
        }).collect::<Vec<OutputFile>>())})
            .to_string(),
        );
        // "return as JSON" is suggested by https://ollama.com/blog/structured-outputs
        request.push("return as JSON".into());
        let request: Vec<_> = request
            .iter()
            .map(|contents| ChatMessage::user().content(contents).build())
            .collect();

        // Make the LLM call.
        log::trace!("Making LLM call with {:?}", request);
        let response = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("tokio failed")
            .block_on(llm.chat(&request))?
            .text()
            .expect("no response text");

        // Parse the response, convert it into a CargoPackage representation.
        #[derive(Deserialize)]
        struct OutputFiles {
            files: Vec<OutputFile>,
        }
        log::trace!("LLM responded: {:?}", &response);
        let files: OutputFiles = serde_json::from_str(&response)?;
        log::info!("LLM response contains {} files.", files.files.len());
        let mut out_dir = RawDir::default();
        for file in files.files {
            out_dir.set_file(&file.path, file.contents.into())?;
        }
        context
            .ir_edit
            .add_representation(Representation::CargoPackage(out_dir));
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct ApiKey(String);

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("********")
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Hostname and port at which to find the LLM serve. Example: "http://[::1]:11434"
    address: Option<String>,

    /// API Key for the LLM service.
    api_key: Option<ApiKey>,

    /// Which backend to use, e.g. "ollama".
    backend: String,

    /// Name of the model to invoke.
    model: String,

    /// Maximum output tokens.
    max_tokens: u32,

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
#[derive(Debug, Deserialize, Serialize)]
struct OutputFile {
    contents: String,
    path: PathBuf,
}
