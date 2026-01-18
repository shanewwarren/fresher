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
