//! Attempts to directly turn a C project into a Cargo project by throwing it at
//! an LLM via the `llm` crate.

use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::{ChatMessage, StructuredOutputFormat};
use serde::Deserialize;
use std::{env, path::PathBuf, str::FromStr};

pub struct RawSourceToCargoLlm;

impl Tool for RawSourceToCargoLlm {
    fn might_write(&mut self, ir: &HarvestIR) -> Option<Vec<Id>> {
        // We need a raw_source to be available, but we won't write any existing IDs.
        raw_source(ir).map(|_| vec![])
    }

    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>> {
        let in_dir = raw_source(&context.ir_snapshot).unwrap();

        // Use the llm crate to connect to Ollama.

        // TODO: This address belongs in a config file (issue #14).
        let llm_backend =
            LLMBackend::from_str(env::var("LLM_BACKEND").as_deref().unwrap_or("ollama"))
                .expect("Unknown LLM_BACKEND");
        let ollama_addr = env::var("OLLAMA_ADDR").unwrap_or("[::1]:11434".into());
        let output_format: StructuredOutputFormat = serde_json::from_str(STRUCTURED_OUTPUT_SCHEMA)?;
        let llm = LLMBuilder::new()
            .backend(llm_backend)
            .base_url(format!("http://{ollama_addr}"))
            .model(env::var("LLM_MODEL").unwrap_or("codellama:7b".into()))
            .max_tokens(100000)
            .temperature(0.0) // Suggestion from https://ollama.com/blog/structured-outputs
            .schema(output_format)
            .system("You are a code translation tool. Please translate the provided C project into a Rust project including Cargo manifest.")
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

/// Structured output JSON schema for Ollama.
const STRUCTURED_OUTPUT_SCHEMA: &str = r#"{
    "name": "file",
    "schema": {
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "contents": { "type": "string"}
            },
            "required": ["path", "contents"]
        }
    }
}"#;
