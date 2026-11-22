//! Trimmed native records, shaped like real Claude Code and Codex files.

pub(super) const CLAUDE: &str = concat!(
    r#"{"type":"session-owner","sessionId":"0b5c7d1e"}"#,
    "\n",
    r#"{"type":"user","isSidechain":false,"sessionId":"0b5c7d1e","cwd":"/home/me/code/app","gitBranch":"fix/login","version":"2.1.300","timestamp":"2026-11-17T10:00:00.000Z","message":{"role":"user","content":"Fix the login bug\nIt fails on Safari."}}"#,
    "\n",
    r#"{"type":"assistant","isSidechain":false,"timestamp":"2026-11-17T10:00:05.000Z","message":{"id":"msg_1","model":"claude-opus-5-5","role":"assistant","content":[{"type":"thinking","thinking":"Look at the form."}],"usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":50}}}"#,
    "\n",
    r#"{"type":"assistant","isSidechain":false,"timestamp":"2026-11-17T10:00:06.000Z","message":{"id":"msg_1","model":"claude-opus-5-5","role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"npm test"}}],"usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":50}}}"#,
    "\n",
    r#"{"type":"user","isSidechain":false,"timestamp":"2026-11-17T10:00:09.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","is_error":true,"content":"1 failing"}]}}"#,
    "\n",
    r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-11-17T10:00:10.000Z","message":{"id":"msg_s","role":"assistant","content":[{"type":"text","text":"subagent"}]}}"#,
    "\n",
    r#"{"type":"assistant","isSidechain":false,"timestamp":"2026-11-17T10:00:20.000Z","message":{"id":"msg_2","model":"<synthetic>","role":"assistant","content":[{"type":"text","text":"Fixed: the cookie was SameSite=Strict."}]}}"#,
    "\n",
    r#"{"type":"user","isSidechain":false,"timestamp":"2026-11-17T10:01:00.000Z","message":{"role":"user","content":[{"type":"text","text":"Thanks, open a PR"}]}}"#,
    "\n",
);

pub(super) const CLAUDE_MORE: &str = concat!(
    r#"{"type":"user","isSidechain":false,"timestamp":"2026-11-17T10:05:00.000Z","message":{"role":"user","content":"And add a test"}}"#,
    "\n",
);

pub(super) const CLAUDE_SECRET: &str = r#"{"type":"user","message":{"role":"user","content":"Use token ghp_abcdefghijklmnopqrstuvwxyz0123456789 in /home/me/code/app"}}"#;

pub(super) const CODEX: &str = concat!(
    r#"{"timestamp":"2026-11-17T10:00:00.000Z","type":"session_meta","payload":{"id":"019a-codex","cwd":"/home/me/code/api","cli_version":"0.90.0","git":{"branch":"main"}}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:00.100Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>cwd</environment_context>"}]}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:01.000Z","type":"turn_context","payload":{"model":"gpt-5.5-codex","effort":"high"}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:01.500Z","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Add a health check"}]}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:02.000Z","type":"event_msg","payload":{"type":"user_message","message":"Add a health check"}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:03.000Z","type":"response_item","payload":{"type":"reasoning","summary":[{"type":"summary_text","text":"Find the router."}]}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:04.000Z","type":"response_item","payload":{"type":"function_call","name":"shell","call_id":"c1","arguments":"{\"command\":[\"rg\",\"router\"]}"}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:05.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":{"content":"src/router.rs","success":true}}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:06.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":900,"output_tokens":120}}}}"#,
    "\n",
    r#"{"timestamp":"2026-11-17T10:00:07.000Z","type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Added GET /health."}]}}"#,
    "\n",
);

pub(super) const PI: &str = concat!(
    r#"{"type":"session","version":3,"id":"7f3a-pi","timestamp":"2026-11-17T09:00:00.000Z","cwd":"/home/me/code/site"}"#,
    "\n",
    r#"{"type":"message","id":"s1","parentId":null,"timestamp":"2026-11-17T09:00:00.100Z","message":{"role":"system","content":"","sections":{"preamble":"You are an expert coding assistant"},"timestamp":0}}"#,
    "\n",
    r#"{"type":"message","id":"u1","parentId":"s1","timestamp":"2026-11-17T09:00:01.000Z","message":{"role":"user","content":"Make the header sticky","timestamp":0}}"#,
    "\n",
    r#"{"type":"message","id":"a1","parentId":"u1","timestamp":"2026-11-17T09:00:03.000Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"CSS position."},{"type":"toolCall","id":"c1","name":"edit","arguments":{"path":"style.css"}}],"provider":"anthropic","model":"claude-opus-5-5","usage":{"input":20,"output":30,"cacheRead":200,"cacheWrite":5,"totalTokens":255},"stopReason":"toolUse","timestamp":0}}"#,
    "\n",
    r#"{"type":"message","id":"r1","parentId":"a1","timestamp":"2026-11-17T09:00:04.000Z","message":{"role":"toolResult","toolCallId":"c1","toolName":"edit","content":[{"type":"text","text":"edited"}],"isError":false,"timestamp":0}}"#,
    "\n",
    r#"{"type":"usage","id":"w1","parentId":"r1","timestamp":"2026-11-17T09:00:04.500Z","kind":"cache_warm","usage":{"input":0,"output":0,"cacheRead":1000,"cacheWrite":0}}"#,
    "\n",
    r#"{"type":"model_change","id":"m1","parentId":"w1","timestamp":"2026-11-17T09:00:05.000Z","provider":"openai","modelId":"gpt-5.5"}"#,
    "\n",
    r#"{"type":"message","id":"a2","parentId":"m1","timestamp":"2026-11-17T09:00:06.000Z","message":{"role":"assistant","content":[{"type":"text","text":"The header is sticky now."}],"model":"gpt-5.5","usage":{"input":1,"output":4,"cacheRead":0,"cacheWrite":0},"stopReason":"stop","timestamp":0}}"#,
    "\n",
    r#"{"type":"message","id":"a3","parentId":"a2","timestamp":"2026-11-17T09:00:07.000Z","message":{"role":"assistant","content":[],"model":"gpt-5.5","usage":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0},"stopReason":"error","errorMessage":"rate limited","timestamp":0}}"#,
    "\n",
    r#"{"type":"session_info","id":"n1","parentId":"a3","timestamp":"2026-11-17T09:00:08.000Z","name":"Sticky header"}"#,
    "\n",
);
