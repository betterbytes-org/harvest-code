//! Individual tools (and their interfaces) used by HARVEST to translate C to Rust.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Combined configuration for all Tools in this crate.
#[derive(Debug, Deserialize)]
pub struct ToolConfigs {
    #[serde(flatten)]
    pub config: HashMap<String, Value>,
}

impl ToolConfigs {
    pub fn validate(&self) {
        //unknown_field_warning("tools", &self.unknown);
        //self.raw_source_to_cargo_llm.validate();
    }

    /// Returns a mock config for testing.
    pub fn mock() -> Self {
        Self {
            //raw_source_to_cargo_llm: raw_source_to_cargo_llm::Config::mock(),
            config: HashMap::new(),
        }
    }
}
