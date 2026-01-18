use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};

/// Event types from Claude Code stream-json output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum StreamEvent {
    /// System information
    System(SystemEvent),
    /// Assistant message
    Assistant(AssistantEvent),
    /// User message (tool results)
    User(UserEvent),
    /// Content block start (tool starting)
    ContentBlockStart(ContentBlockStartEvent),
    /// Content block delta (streaming)
    ContentBlockDelta(ContentBlockDeltaEvent),
    /// Content block stop (tool complete)
    ContentBlockStop(ContentBlockStopEvent),
    /// Final result
    Result(ResultEvent),
    /// Unknown event type
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub subtype: Option<String>,
    pub session_id: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantEvent {
    pub message: Option<AssistantMessage>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEvent {
    pub message: Option<UserMessage>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<UserContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum UserContentBlock {
    ToolResult { tool_use_id: String, content: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockStartEvent {
    pub index: Option<usize>,
    pub content_block: Option<ContentBlock>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent {
    pub index: Option<usize>,
    pub delta: Option<Delta>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Delta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlockStopEvent {
    pub index: Option<usize>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultEvent {
    pub subtype: Option<String>,
    pub is_error: Option<bool>,
    pub duration_ms: Option<u64>,
    pub duration_api_ms: Option<u64>,
    pub num_turns: Option<u32>,
    pub result: Option<String>,
    pub cost_usd: Option<f64>,
    pub session_id: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

/// Parse a single JSON line into a StreamEvent
pub fn parse_event(line: &str) -> Result<StreamEvent> {
    serde_json::from_str(line).context("Failed to parse stream event")
}

/// Stream handler for processing Claude Code output
pub struct StreamHandler {
    pub show_tool_calls: bool,
    pub show_tool_results: bool,
    pub show_text: bool,
    pub verbose: bool,
}

impl Default for StreamHandler {
    fn default() -> Self {
        Self {
            show_tool_calls: true,
            show_tool_results: false,
            show_text: true,
            verbose: false,
        }
    }
}

impl StreamHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Process a single event and print appropriate output
    pub fn handle_event(&self, event: &StreamEvent) {
        match event {
            StreamEvent::System(e) => {
                if self.verbose {
                    if let Some(subtype) = &e.subtype {
                        println!("{} {}", "[system]".dimmed(), subtype.dimmed());
                    }
                }
            }
            StreamEvent::Assistant(e) => {
                if let Some(msg) = &e.message {
                    for block in &msg.content {
                        match block {
                            ContentBlock::Text { text } => {
                                if self.show_text && !text.is_empty() {
                                    println!("{}", text);
                                }
                            }
                            ContentBlock::ToolUse { name, input, .. } => {
                                if self.show_tool_calls {
                                    self.print_tool_call(name, input);
                                }
                            }
                            ContentBlock::Other => {}
                        }
                    }
                }
            }
            StreamEvent::User(e) => {
                if self.show_tool_results {
                    if let Some(msg) = &e.message {
                        for block in &msg.content {
                            if let UserContentBlock::ToolResult { content, .. } = block {
                                let preview = if content.len() > 200 {
                                    format!("{}...", &content[..200])
                                } else {
                                    content.clone()
                                };
                                println!("  {} {}", "→".dimmed(), preview.dimmed());
                            }
                        }
                    }
                }
            }
            StreamEvent::ContentBlockStart(e) => {
                if self.verbose {
                    if let Some(block) = &e.content_block {
                        if let ContentBlock::ToolUse { name, .. } = block {
                            println!("  {} {}", "starting:".dimmed(), name.yellow());
                        }
                    }
                }
            }
            StreamEvent::ContentBlockDelta(_) => {
                // Streaming deltas - usually not printed in loop mode
            }
            StreamEvent::ContentBlockStop(_) => {
                // Block complete
            }
            StreamEvent::Result(e) => {
                if let Some(result) = &e.result {
                    if !result.is_empty() && self.show_text {
                        println!("\n{}", result);
                    }
                }
                if self.verbose {
                    if let Some(duration) = e.duration_ms {
                        println!(
                            "\n{} {}ms",
                            "Duration:".dimmed(),
                            duration.to_string().cyan()
                        );
                    }
                    if let Some(cost) = e.cost_usd {
                        println!("{} ${:.4}", "Cost:".dimmed(), cost);
                    }
                    if let Some(turns) = e.num_turns {
                        println!("{} {}", "Turns:".dimmed(), turns);
                    }
                }
            }
            StreamEvent::Unknown => {
                if self.verbose {
                    println!("{}", "[unknown event]".dimmed());
                }
            }
        }
    }

    fn print_tool_call(&self, name: &str, input: &Value) {
        let formatted = match name {
            "Bash" => {
                if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                    let preview = if cmd.len() > 100 {
                        format!("{}...", &cmd[..100])
                    } else {
                        cmd.to_string()
                    };
                    format!("{} {}", "Bash:".blue().bold(), preview)
                } else {
                    format!("{}", "Bash".blue().bold())
                }
            }
            "Read" => {
                if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                    format!("{} {}", "Read:".green().bold(), path)
                } else {
                    format!("{}", "Read".green().bold())
                }
            }
            "Write" => {
                if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                    format!("{} {}", "Write:".yellow().bold(), path)
                } else {
                    format!("{}", "Write".yellow().bold())
                }
            }
            "Edit" => {
                if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                    format!("{} {}", "Edit:".yellow().bold(), path)
                } else {
                    format!("{}", "Edit".yellow().bold())
                }
            }
            "Glob" | "Grep" => {
                if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                    format!("{} {}", format!("{}:", name).cyan().bold(), pattern)
                } else {
                    format!("{}", name.cyan().bold())
                }
            }
            "Task" => {
                if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
                    format!("{} {}", "Task:".magenta().bold(), desc)
                } else {
                    format!("{}", "Task".magenta().bold())
                }
            }
            "TodoWrite" => {
                format!("{}", "TodoWrite".cyan().bold())
            }
            _ => {
                format!("{}", name.bold())
            }
        };
        println!("  {} {}", "→".dimmed(), formatted);
    }
}

/// Process result summary
#[derive(Debug, Default)]
pub struct ProcessResult {
    pub exit_code: i32,
    pub duration_ms: Option<u64>,
    pub cost_usd: Option<f64>,
    pub num_turns: Option<u32>,
    pub is_error: bool,
    pub result_text: Option<String>,
}

/// Process Claude Code stream output and return summary
pub async fn process_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    handler: &StreamHandler,
) -> Result<ProcessResult> {
    let mut buf_reader = AsyncBufReader::new(reader);
    let mut line = String::new();
    let mut result = ProcessResult::default();

    loop {
        line.clear();
        let bytes_read = buf_reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match parse_event(trimmed) {
            Ok(event) => {
                handler.handle_event(&event);

                // Capture result info
                if let StreamEvent::Result(ref e) = event {
                    result.duration_ms = e.duration_ms;
                    result.cost_usd = e.cost_usd;
                    result.num_turns = e.num_turns;
                    result.is_error = e.is_error.unwrap_or(false);
                    result.result_text = e.result.clone();
                }
            }
            Err(e) => {
                if handler.verbose {
                    eprintln!("Warning: failed to parse event: {}", e);
                    eprintln!("  Line: {}", trimmed);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_system() {
        let json = r#"{"type":"system","subtype":"init","session_id":"abc123"}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::System(e) => {
                assert_eq!(e.subtype, Some("init".to_string()));
                assert_eq!(e.session_id, Some("abc123".to_string()));
            }
            _ => panic!("Expected System event"),
        }
    }

    #[test]
    fn test_parse_event_assistant_text() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::Assistant(e) => {
                let msg = e.message.unwrap();
                assert_eq!(msg.content.len(), 1);
                match &msg.content[0] {
                    ContentBlock::Text { text } => assert_eq!(text, "Hello world"),
                    _ => panic!("Expected Text block"),
                }
            }
            _ => panic!("Expected Assistant event"),
        }
    }

    #[test]
    fn test_parse_event_assistant_tool_use() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool1","name":"Bash","input":{"command":"ls -la"}}]}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::Assistant(e) => {
                let msg = e.message.unwrap();
                assert_eq!(msg.content.len(), 1);
                match &msg.content[0] {
                    ContentBlock::ToolUse { id, name, input } => {
                        assert_eq!(id, "tool1");
                        assert_eq!(name, "Bash");
                        assert_eq!(input["command"].as_str().unwrap(), "ls -la");
                    }
                    _ => panic!("Expected ToolUse block"),
                }
            }
            _ => panic!("Expected Assistant event"),
        }
    }

    #[test]
    fn test_parse_event_user_tool_result() {
        let json = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tool1","content":"output text"}]}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::User(e) => {
                let msg = e.message.unwrap();
                assert_eq!(msg.content.len(), 1);
                match &msg.content[0] {
                    UserContentBlock::ToolResult { tool_use_id, content } => {
                        assert_eq!(tool_use_id, "tool1");
                        assert_eq!(content, "output text");
                    }
                    _ => panic!("Expected ToolResult block"),
                }
            }
            _ => panic!("Expected User event"),
        }
    }

    #[test]
    fn test_parse_event_content_block_start() {
        let json = r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool1","name":"Read","input":{}}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::ContentBlockStart(e) => {
                assert_eq!(e.index, Some(0));
                match e.content_block.unwrap() {
                    ContentBlock::ToolUse { name, .. } => assert_eq!(name, "Read"),
                    _ => panic!("Expected ToolUse block"),
                }
            }
            _ => panic!("Expected ContentBlockStart event"),
        }
    }

    #[test]
    fn test_parse_event_content_block_delta_text() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"partial output"}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::ContentBlockDelta(e) => {
                assert_eq!(e.index, Some(0));
                match e.delta.unwrap() {
                    Delta::TextDelta { text } => assert_eq!(text, "partial output"),
                    _ => panic!("Expected TextDelta"),
                }
            }
            _ => panic!("Expected ContentBlockDelta event"),
        }
    }

    #[test]
    fn test_parse_event_content_block_delta_json() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"key\":"}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::ContentBlockDelta(e) => {
                match e.delta.unwrap() {
                    Delta::InputJsonDelta { partial_json } => assert_eq!(partial_json, r#"{"key":"#),
                    _ => panic!("Expected InputJsonDelta"),
                }
            }
            _ => panic!("Expected ContentBlockDelta event"),
        }
    }

    #[test]
    fn test_parse_event_content_block_stop() {
        let json = r#"{"type":"content_block_stop","index":0}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::ContentBlockStop(e) => {
                assert_eq!(e.index, Some(0));
            }
            _ => panic!("Expected ContentBlockStop event"),
        }
    }

    #[test]
    fn test_parse_event_result() {
        let json = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":5000,"duration_api_ms":4500,"num_turns":10,"result":"Task completed","cost_usd":0.05,"session_id":"xyz789"}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::Result(e) => {
                assert_eq!(e.subtype, Some("success".to_string()));
                assert_eq!(e.is_error, Some(false));
                assert_eq!(e.duration_ms, Some(5000));
                assert_eq!(e.duration_api_ms, Some(4500));
                assert_eq!(e.num_turns, Some(10));
                assert_eq!(e.result, Some("Task completed".to_string()));
                assert_eq!(e.cost_usd, Some(0.05));
                assert_eq!(e.session_id, Some("xyz789".to_string()));
            }
            _ => panic!("Expected Result event"),
        }
    }

    #[test]
    fn test_parse_event_unknown() {
        let json = r#"{"type":"some_new_type","data":"value"}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::Unknown => {}
            _ => panic!("Expected Unknown event"),
        }
    }

    #[test]
    fn test_parse_event_invalid_json() {
        let result = parse_event("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_handler_default() {
        let handler = StreamHandler::default();
        assert!(handler.show_tool_calls);
        assert!(!handler.show_tool_results);
        assert!(handler.show_text);
        assert!(!handler.verbose);
    }

    #[test]
    fn test_stream_handler_new() {
        let handler = StreamHandler::new();
        assert!(handler.show_tool_calls);
        assert!(!handler.verbose);
    }

    #[test]
    fn test_stream_handler_verbose() {
        let handler = StreamHandler::new().verbose(true);
        assert!(handler.verbose);
    }

    #[test]
    fn test_process_result_default() {
        let result = ProcessResult::default();
        assert_eq!(result.exit_code, 0);
        assert!(result.duration_ms.is_none());
        assert!(result.cost_usd.is_none());
        assert!(result.num_turns.is_none());
        assert!(!result.is_error);
        assert!(result.result_text.is_none());
    }

    #[tokio::test]
    async fn test_process_stream_empty() {
        let data = b"";
        let handler = StreamHandler::new();
        let result = process_stream(&data[..], &handler).await.unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.duration_ms.is_none());
    }

    #[tokio::test]
    async fn test_process_stream_with_result() {
        let data = b"{\"type\":\"result\",\"duration_ms\":1000,\"cost_usd\":0.01,\"num_turns\":5,\"is_error\":false}\n";
        let handler = StreamHandler::new();
        let result = process_stream(&data[..], &handler).await.unwrap();

        assert_eq!(result.duration_ms, Some(1000));
        assert_eq!(result.cost_usd, Some(0.01));
        assert_eq!(result.num_turns, Some(5));
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_process_stream_skips_empty_lines() {
        let data = b"\n\n{\"type\":\"result\",\"duration_ms\":500}\n\n";
        let handler = StreamHandler::new();
        let result = process_stream(&data[..], &handler).await.unwrap();

        assert_eq!(result.duration_ms, Some(500));
    }

    #[tokio::test]
    async fn test_process_stream_handles_invalid_json() {
        let data = b"invalid json\n{\"type\":\"result\",\"duration_ms\":100}\n";
        let handler = StreamHandler::new().verbose(false);
        let result = process_stream(&data[..], &handler).await.unwrap();

        // Should still capture the valid result event
        assert_eq!(result.duration_ms, Some(100));
    }

    #[tokio::test]
    async fn test_process_stream_error_result() {
        let data = b"{\"type\":\"result\",\"is_error\":true}\n";
        let handler = StreamHandler::new();
        let result = process_stream(&data[..], &handler).await.unwrap();

        assert!(result.is_error);
    }

    #[test]
    fn test_parse_event_assistant_multiple_blocks() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"First"},{"type":"text","text":"Second"}]}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::Assistant(e) => {
                let msg = e.message.unwrap();
                assert_eq!(msg.content.len(), 2);
            }
            _ => panic!("Expected Assistant event"),
        }
    }

    #[test]
    fn test_content_block_other() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"some_unknown_type"}]}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::Assistant(e) => {
                let msg = e.message.unwrap();
                assert_eq!(msg.content.len(), 1);
                assert!(matches!(msg.content[0], ContentBlock::Other));
            }
            _ => panic!("Expected Assistant event"),
        }
    }

    #[test]
    fn test_delta_other() {
        let json = r#"{"type":"content_block_delta","index":0,"delta":{"type":"some_unknown_delta"}}"#;
        let event = parse_event(json).unwrap();

        match event {
            StreamEvent::ContentBlockDelta(e) => {
                assert!(matches!(e.delta.unwrap(), Delta::Other));
            }
            _ => panic!("Expected ContentBlockDelta event"),
        }
    }
}
