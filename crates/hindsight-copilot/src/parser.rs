//! JSON stream parser for Copilot logs

use serde_json::StreamDeserializer;
use std::io::Read;

use crate::error::CopilotError;
use crate::lsp::LspMessage;

/// Parser for JSON stream formatted Copilot logs
pub struct LogParser<R: Read> {
    deserializer: StreamDeserializer<'static, serde_json::de::IoRead<R>, LspMessage>,
}

impl<R: Read> LogParser<R> {
    /// Create a new log parser from a reader
    pub fn new(reader: R) -> Self {
        let deserializer = serde_json::Deserializer::from_reader(reader).into_iter();
        Self { deserializer }
    }
}

impl<R: Read> Iterator for LogParser<R> {
    type Item = Result<LspMessage, CopilotError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.deserializer
            .next()
            .map(|result| result.map_err(CopilotError::from))
    }
}

// TODO: Implement additional parsing utilities
