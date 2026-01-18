// Copyright (c) 2026 - present Nicholas D. Crosbie
// SPDX-License-Identifier: MIT

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

/// Parse all LSP messages from a JSON string
///
/// # Errors
///
/// Returns an error if the JSON is invalid or doesn't match the expected format.
pub fn parse_json_stream(json: &str) -> Result<Vec<LspMessage>, CopilotError> {
    let mut parser = LogParser::new(json.as_bytes());
    let mut messages = Vec::new();

    for result in &mut parser {
        messages.push(result?);
    }

    Ok(messages)
}

/// Parse a single LSP message from a JSON string
///
/// # Errors
///
/// Returns an error if the JSON is invalid.
pub fn parse_single_message(json: &str) -> Result<LspMessage, CopilotError> {
    serde_json::from_str(json).map_err(CopilotError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar_asserts::assert_eq;
    use std::io::Cursor;

    #[test]
    fn test_log_parser_single_message() {
        let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let reader = Cursor::new(json);
        let mut parser = LogParser::new(reader);

        let msg = parser
            .next()
            .expect("should have message")
            .expect("should parse");
        assert_eq!(msg.jsonrpc, "2.0");
        assert_eq!(msg.method, Some("test".to_string()));

        assert!(parser.next().is_none());
    }

    #[test]
    fn test_log_parser_multiple_messages() {
        let json = r#"{"jsonrpc":"2.0","method":"first","id":1}
{"jsonrpc":"2.0","method":"second","id":2}"#;
        let reader = Cursor::new(json);
        let parser = LogParser::new(reader);

        let messages: Vec<_> = parser.collect();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].is_ok());
        assert!(messages[1].is_ok());
    }

    #[test]
    fn test_log_parser_empty_input() {
        let json = "";
        let reader = Cursor::new(json);
        let mut parser = LogParser::new(reader);

        assert!(parser.next().is_none());
    }

    #[test]
    fn test_log_parser_invalid_json() {
        let json = r#"{"jsonrpc":"2.0" invalid"#;
        let reader = Cursor::new(json);
        let mut parser = LogParser::new(reader);

        let result = parser.next().expect("should have result");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_stream() {
        let json = r#"{"jsonrpc":"2.0","method":"a","id":1}{"jsonrpc":"2.0","method":"b","id":2}"#;
        let messages = parse_json_stream(json).expect("should parse");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].method, Some("a".to_string()));
        assert_eq!(messages[1].method, Some("b".to_string()));
    }

    #[test]
    fn test_parse_json_stream_empty() {
        let messages = parse_json_stream("").expect("should parse");
        assert!(messages.is_empty());
    }

    #[test]
    fn test_parse_single_message() {
        let json = r#"{"jsonrpc":"2.0","method":"test","id":1,"params":{"uri":"file:///test.rs"}}"#;
        let msg = parse_single_message(json).expect("should parse");

        assert_eq!(msg.jsonrpc, "2.0");
        assert_eq!(msg.method, Some("test".to_string()));
        assert!(msg.params.is_some());
    }

    #[test]
    fn test_parse_single_message_invalid() {
        let result = parse_single_message("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_message_with_all_fields() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 42,
            "method": "textDocument/completion",
            "params": {"position": {"line": 10, "character": 5}},
            "result": {"completions": []},
            "error": null
        }"#;

        let msg = parse_single_message(json).expect("should parse");
        assert_eq!(msg.id, Some(serde_json::json!(42)));
        assert_eq!(msg.method, Some("textDocument/completion".to_string()));
        assert!(msg.params.is_some());
    }
}
