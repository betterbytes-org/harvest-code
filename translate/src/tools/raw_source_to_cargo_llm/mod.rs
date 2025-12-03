//! Attempts to directly turn a C project into a Cargo project by throwing it at
//! an LLM via the `llm` crate.

use crate::cli::unknown_field_warning;
use crate::load_raw_source::RawSource;
use crate::tools::{MightWriteContext, MightWriteOutcome, RunContext, Tool};
use harvest_ir::{HarvestIR, Representation, fs::RawDir};
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::{ChatMessage, StructuredOutputFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

/// Structured output JSON schema for Ollama.
const STRUCTURED_OUTPUT_SCHEMA: &str = include_str!("structured_schema.json");

const SYSTEM_PROMPT: &str = include_str!("system_prompt.txt");

/// Builds an LLM instance with the provided configuration.
pub fn build_llm(config: &Config) -> Result<Box<dyn llm::LLMProvider>, Box<dyn std::error::Error>> {
    let output_format: StructuredOutputFormat = serde_json::from_str(STRUCTURED_OUTPUT_SCHEMA)?;

    // TODO: This is a workaround for a flaw in the current
    // version (1.3.4) of the `llm` crate. While it supports
    // OpenRouter, the `openrouter` variant hadn't been added to
    // `from_str`. It's fixed on git tip, but not in a release
    // version. So just check for that case explicitly.
    let backend = if config.backend == "openrouter" {
        LLMBackend::OpenRouter
    } else {
        LLMBackend::from_str(&config.backend).expect("unknown LLM_BACKEND")
    };

    let mut llm_builder = LLMBuilder::new()
        .backend(backend)
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

    Ok(llm_builder.build().expect("Failed to build LLM"))
}

/// Builds the LLM translation request.
/// If this is the first time attempting translation, provide the source files.
/// Otherwise, provide the last compiler error.
fn build_initial_translation_request(in_dir: &RawDir) -> Vec<ChatMessage> {
    let mut request: Vec<String> = vec![
        "Please translate the following C project into a Rust project including Cargo manifest:"
            .into(),
    ];
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
    request
        .iter()
        .map(|contents| ChatMessage::user().content(contents).build())
        .collect()
}

fn build_retry_request(last_error: &str) -> Vec<ChatMessage> {
    let request: Vec<String> = vec![
        "The previous Rust project you generated failed to compile with the following error:"
            .into(),
        last_error.into(),
        "Please fix the Rust project accordingly and return the updated files as JSON.".into(),
    ];
    request
        .iter()
        .map(|contents| ChatMessage::user().content(contents).build())
        .collect()
}

pub struct RawSourceToCargoLlm {
    llm: Arc<dyn llm::LLMProvider>,
    previous_build_results: Vec<String>,
}
impl RawSourceToCargoLlm {
    pub fn new(llm: Arc<dyn llm::LLMProvider>, previous_build_results: &[String]) -> Self {
        RawSourceToCargoLlm {
            llm,
            previous_build_results: previous_build_results.to_vec(),
        }
    }
}

impl Tool for RawSourceToCargoLlm {
    fn name(&self) -> &'static str {
        "RawSourceToCargoLlm"
    }

    fn might_write(&mut self, context: MightWriteContext) -> MightWriteOutcome {
        // We need a raw_source to be available, but we won't write any existing IDs.
        match raw_source(context.ir) {
            None => MightWriteOutcome::TryAgain,
            Some(_) => MightWriteOutcome::Runnable([].into()),
        }
    }

    fn run(self: Box<Self>, context: RunContext) -> Result<(), Box<dyn std::error::Error>> {
        let config = &context.config.tools.raw_source_to_cargo_llm;
        log::debug!("LLM Configuration {config:?}");
        let in_dir = raw_source(&context.ir_snapshot).unwrap();

        // Build the llm translation request.
        let request = match self.previous_build_results.last() {
            Some(last_error) => {
                // There was a previous build error - provide it to the LLM for context.
                build_retry_request(last_error)
            }
            None => {
                // No previous build errors - this is the initial translation attempt.
                build_initial_translation_request(in_dir)
            }
        };

        // Make the LLM call.
        log::trace!("Making LLM call with {:?}", request);
        let response = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("tokio failed")
            .block_on(self.llm.chat(&request))?
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
            .add_representation(Box::new(CargoPackage { dir: out_dir }));
        Ok(())
    }
}

/// A cargo project representation (Cargo.toml, src/, etc).
pub struct CargoPackage {
    pub dir: RawDir,
}

impl std::fmt::Display for CargoPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Cargo package:")?;
        self.dir.display(0, f)
    }
}

impl Representation for CargoPackage {
    fn name(&self) -> &'static str {
        "CargoPackage"
    }

    fn materialize(&self, path: &Path) -> std::io::Result<()> {
        self.dir.materialize(path)
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
    pub backend: String,

    /// Name of the model to invoke.
    pub model: String,

    /// Maximum output tokens.
    pub max_tokens: u32,

    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}

impl Config {
    pub fn validate(&self) {
        unknown_field_warning("tools.raw_source_to_cargo_llm", &self.unknown);
    }

    /// Returns a mock config for testing.
    pub fn mock() -> Self {
        Self {
            address: None,
            api_key: None,
            backend: "mock_llm".into(),
            model: "mock_model".into(),
            max_tokens: 1000,
            unknown: HashMap::new(),
        }
    }
}

/// Returns the RawSource representation in IR. If there are multiple RawSource representations,
/// returns an arbitrary one.
fn raw_source(ir: &HarvestIR) -> Option<&RawDir> {
    ir.get_by_representation::<RawSource>()
        .next()
        .map(|(_, r)| &r.dir)
}

/// Structure representing a file created by the LLM.
#[derive(Debug, Deserialize, Serialize)]
struct OutputFile {
    contents: String,
    path: PathBuf,
}
