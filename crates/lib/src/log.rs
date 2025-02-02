use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum LoggingFormat {
    Standard,
    Json,
}
