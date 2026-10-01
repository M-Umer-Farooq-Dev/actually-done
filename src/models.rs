use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, String>;
pub type Usage = BTreeMap<String, u64>;

#[derive(Clone, Default, Debug)]
pub struct Tool {
    pub name: String,
    pub input: Value,
    pub id: String,
    pub success: Option<bool>,
    pub is_error: bool,
}

#[derive(Clone, Default, Debug)]
pub struct Event {
    pub ts: Option<String>,
    pub role: String,
    pub text: String,
    pub tools: Vec<Tool>,
    pub model: Option<String>,
    pub final_response: bool,
}

#[derive(Default, Debug)]
pub struct Session {
    pub provider: String,
    pub id: String,
    pub path: String,
    pub cwd: Option<String>,
    pub started: Option<String>,
    pub ended: Option<String>,
    pub models: Vec<String>,
    pub events: Vec<Event>,
    pub prompts: Vec<String>,
    pub final_text: String,
    pub pending_user: bool,
    pub usage: Usage,
    pub warnings: usize,
    pub notes: Vec<String>,
}

#[derive(Serialize, Default, Debug)]
pub struct GitInfo {
    pub root: String,
    pub branch: Option<String>,
    pub changed_files: Vec<String>,
    pub staged: Vec<String>,
    pub unstaged: Vec<String>,
    pub untracked: Vec<String>,
    pub error: Option<String>,
    pub disabled: bool,
}

#[derive(Serialize, Debug)]
pub struct TestResult {
    pub status: String,
    pub command: Option<String>,
    pub returncode: Option<i32>,
    pub output: String,
    pub error: Option<String>,
    pub agent_activity: Vec<String>,
}
impl Default for TestResult {
    fn default() -> Self {
        Self {
            status: "unverified".into(),
            command: None,
            returncode: None,
            output: String::new(),
            error: None,
            agent_activity: vec![],
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub text: String,
    pub source: String,
    pub evidence_line: String,
}
impl Finding {
    pub fn new(text: &str, source: &str) -> Self {
        Self {
            text: text.into(),
            source: source.into(),
            evidence_line: text.into(),
        }
    }
}

#[derive(Default)]
pub struct Claims {
    pub done: bool,
    pub files: Vec<String>,
    pub read: Vec<String>,
    pub written: Vec<String>,
    pub edited: Vec<String>,
    pub attempted: Vec<String>,
    pub leftovers: Vec<Finding>,
    pub scope: Vec<Finding>,
    pub activity: Vec<String>,
}

#[derive(Serialize)]
pub struct Receipt {
    pub schema_version: String,
    pub provider: String,
    pub session_id: String,
    pub session_path: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub models: Vec<String>,
    pub event_count: usize,
    pub prompt: String,
    pub prompt_index: usize,
    pub claimed_done: bool,
    pub claimed_files: Vec<String>,
    pub claimed_without_current_change: Vec<String>,
    pub changed_but_unclaimed: Vec<String>,
    pub files_read: Vec<String>,
    pub files_written: Vec<String>,
    pub files_edited: Vec<String>,
    pub attempted_files: Vec<String>,
    pub git: GitInfo,
    pub tests: TestResult,
    pub leftovers: Vec<Finding>,
    pub scope_dropped: Vec<Finding>,
    pub verdict: String,
    pub exit_code: i32,
    pub reasons: Vec<String>,
    pub uncertainty: Vec<String>,
    pub parse_warnings: usize,
    pub token_totals: Usage,
}
