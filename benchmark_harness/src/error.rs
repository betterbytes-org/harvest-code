/// A simple custom Result type to replicate simple `anyhow::Result` functionality.
pub type HarvestResult<T> = Result<T, Box<dyn std::error::Error>>;
