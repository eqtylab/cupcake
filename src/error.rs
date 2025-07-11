use thiserror::Error;

#[derive(Error, Debug)]
pub enum CupcakeError {
    #[error("Policy parsing error: {0}")]
    PolicyParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialization(#[from] toml::de::Error),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Hook event error: {0}")]
    HookEvent(String),

    #[error("CLI error: {0}")]
    Cli(String),

    #[error("Condition error: {0}")]
    Condition(String),

    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CupcakeError>;
