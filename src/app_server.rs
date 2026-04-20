use std::collections::VecDeque;
use std::process::Stdio;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tracing::warn;

use crate::codex::{CodexOutcome, CodexRequest};
use crate::config::{CodexConfig, WorkspaceConfig};
use crate::store::{ApprovalRequestKind, ApprovalRequestRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexApprovalRequest {
    pub request_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub request_kind: ApprovalRequestKind,
    pub summary_text: String,
    pub raw_payload_json: String,
}

pub struct AppServerClient {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    backlog: VecDeque<Value>,
    next_request_id: u64,
    initialized: bool,
}

impl AppServerClient {
    pub async fn spawn(config: &CodexConfig) -> Result<Self> {
        let mut command = Command::new(&config.binary);
        command
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .context("failed to spawn codex app-server")?;
        let stdin = child.stdin.take().context("missing app-server stdin")?;
        let stdout = child.stdout.take().context("missing app-server stdout")?;
        let stderr = child.stderr.take().context("missing app-server stderr")?;
        tokio::spawn(log_app_server_stderr(stderr));

        Ok(Self {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
            backlog: VecDeque::new(),
            next_request_id: 1,
            initialized: false,
        })
    }

    pub async fn start_turn(
        &mut self,
        config: &CodexConfig,
        workspace: &WorkspaceConfig,
        request: CodexRequest,
    ) -> Result<CodexOutcome> {
        self.ensure_initialized().await?;
        let thread = self
            .call(
                "thread/start",
                json!({
                    "model": config.model,
                    "cwd": workspace.path.display().to_string(),
                    "approvalPolicy": config.approval,
                    "approvalsReviewer": "user",
                    "sandbox": config.sandbox,
                    "experimentalRawEvents": false,
                    "persistExtendedHistory": true,
                    "serviceName": "codex-channels",
                }),
            )
            .await?;
        let thread_id = thread_result_thread_id(&thread)?;
        self.start_turn_on_thread(config, workspace, &thread_id, request)
            .await
    }

    pub async fn resume_turn(
        &mut self,
        config: &CodexConfig,
        workspace: &WorkspaceConfig,
        thread_id: &str,
        request: CodexRequest,
    ) -> Result<CodexOutcome> {
        self.ensure_initialized().await?;
        self.call(
            "thread/resume",
            json!({
                "threadId": thread_id,
                "model": config.model,
                "cwd": workspace.path.display().to_string(),
                "approvalPolicy": config.approval,
                "approvalsReviewer": "user",
                "sandbox": config.sandbox,
                "persistExtendedHistory": true,
            }),
        )
        .await?;
        self.start_turn_on_thread(config, workspace, thread_id, request)
            .await
    }

    pub async fn resolve_approval(
        &mut self,
        request: &ApprovalRequestRecord,
        approved: bool,
    ) -> Result<CodexOutcome> {
        let response = approval_response(request, approved)?;
        self.send_json(&json!({
            "id": request.request_id,
            "response": response,
            "result": response,
        }))
        .await?;
        self.read_until_pause_or_completion(&request.thread_id, &request.turn_id)
            .await
    }

    async fn start_turn_on_thread(
        &mut self,
        config: &CodexConfig,
        workspace: &WorkspaceConfig,
        thread_id: &str,
        request: CodexRequest,
    ) -> Result<CodexOutcome> {
        let turn = self
            .call(
                "turn/start",
                json!({
                    "threadId": thread_id,
                    "input": request_to_user_inputs(request),
                    "cwd": workspace.path.display().to_string(),
                    "approvalPolicy": config.approval,
                    "approvalsReviewer": "user",
                    "sandboxPolicy": sandbox_policy_for_workspace(config, workspace),
                    "model": config.model,
                }),
            )
            .await?;
        let turn_id = turn_result_turn_id(&turn)?;
        self.read_until_pause_or_completion(thread_id, &turn_id)
            .await
    }

    async fn ensure_initialized(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        self.call(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "codex-channels",
                    "title": "codex-channels",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "experimentalApi": false,
                    "optOutNotificationMethods": [],
                }
            }),
        )
        .await?;
        self.send_json(&json!({ "method": "initialized" })).await?;
        self.initialized = true;
        Ok(())
    }

    async fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let request_id = format!("client-{}", self.next_request_id);
        self.next_request_id += 1;
        self.send_json(&json!({
            "id": request_id,
            "method": method,
            "params": params,
        }))
        .await?;

        loop {
            let message = self.read_message().await?;
            if response_id(&message).as_deref() == Some(request_id.as_str()) {
                if let Some(error) = message.get("error") {
                    bail!("app-server `{method}` failed: {error}");
                }
                return message
                    .get("result")
                    .or_else(|| message.get("response"))
                    .cloned()
                    .ok_or_else(|| anyhow!("app-server `{method}` missing result/response"));
            }
            self.backlog.push_back(message);
        }
    }

    async fn read_until_pause_or_completion(
        &mut self,
        thread_id: &str,
        turn_id: &str,
    ) -> Result<CodexOutcome> {
        let mut delta_message = String::new();
        let mut approval_requests = Vec::new();
        let mut approval_resolved_count = 0_i64;

        loop {
            let message = self.read_message().await?;
            let Some(method) = message.get("method").and_then(Value::as_str) else {
                self.backlog.push_back(message);
                continue;
            };

            let params = message.get("params").cloned().unwrap_or(Value::Null);
            if message.get("id").is_some() {
                if let Some(request) = parse_approval_request(&message)? {
                    approval_requests.push(request);
                    return Ok(CodexOutcome {
                        session_id: Some(thread_id.to_owned()),
                        turn_id: Some(turn_id.to_owned()),
                        last_message: delta_message,
                        exit_code: None,
                        approval_pending: true,
                        approval_requests,
                        approval_request_count: 1,
                        approval_resolved_count,
                    });
                }
                self.backlog.push_back(message);
                continue;
            }

            match method {
                "item/agentMessage/delta" => {
                    if params.get("threadId").and_then(Value::as_str) == Some(thread_id)
                        && params.get("turnId").and_then(Value::as_str) == Some(turn_id)
                    {
                        if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                            delta_message.push_str(delta);
                        }
                    }
                }
                "serverRequest/resolved" => {
                    if params.get("threadId").and_then(Value::as_str) == Some(thread_id) {
                        approval_resolved_count += 1;
                    }
                }
                "turn/completed" => {
                    let completed_turn = params
                        .get("turn")
                        .ok_or_else(|| anyhow!("turn/completed missing turn"))?;
                    if completed_turn.get("id").and_then(Value::as_str) != Some(turn_id) {
                        continue;
                    }
                    let status = completed_turn
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("failed");
                    let last_message = self
                        .read_last_agent_message(thread_id, turn_id)
                        .await
                        .unwrap_or(delta_message);
                    return Ok(CodexOutcome {
                        session_id: Some(thread_id.to_owned()),
                        turn_id: Some(turn_id.to_owned()),
                        last_message,
                        exit_code: Some(turn_status_exit_code(status)),
                        approval_pending: false,
                        approval_requests,
                        approval_request_count: 0,
                        approval_resolved_count,
                    });
                }
                _ => {}
            }
        }
    }

    async fn read_last_agent_message(&mut self, thread_id: &str, turn_id: &str) -> Result<String> {
        let response = self
            .call(
                "thread/read",
                json!({
                    "threadId": thread_id,
                    "includeTurns": true,
                }),
            )
            .await?;
        let turns = response
            .get("thread")
            .and_then(|thread| thread.get("turns"))
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("thread/read missing turns"))?;

        for turn in turns.iter().rev() {
            if turn.get("id").and_then(Value::as_str) != Some(turn_id) {
                continue;
            }
            if let Some(items) = turn.get("items").and_then(Value::as_array) {
                for item in items.iter().rev() {
                    if item.get("type").and_then(Value::as_str) == Some("agentMessage") {
                        return Ok(item
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned());
                    }
                }
            }
        }
        Ok(String::new())
    }

    async fn read_message(&mut self) -> Result<Value> {
        if let Some(message) = self.backlog.pop_front() {
            return Ok(message);
        }

        loop {
            let mut line = String::new();
            let bytes_read = self
                .stdout
                .read_line(&mut line)
                .await
                .context("failed to read from app-server stdout")?;
            if bytes_read == 0 {
                bail!("app-server closed stdout");
            }
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str(line.trim())
                .context("failed to decode app-server JSON message");
        }
    }

    async fn send_json(&mut self, value: &Value) -> Result<()> {
        let payload =
            serde_json::to_string(value).context("failed to serialize app-server JSON")?;
        self.stdin
            .write_all(payload.as_bytes())
            .await
            .context("failed to write to app-server stdin")?;
        self.stdin
            .write_all(b"\n")
            .await
            .context("failed to terminate app-server JSON line")?;
        self.stdin
            .flush()
            .await
            .context("failed to flush app-server stdin")?;
        Ok(())
    }
}

async fn log_app_server_stderr(stderr: ChildStderr) {
    let mut lines = BufReader::new(stderr).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => warn!("app-server stderr: {line}"),
            Ok(None) => break,
            Err(error) => {
                warn!("failed to read app-server stderr: {error:#}");
                break;
            }
        }
    }
}

fn sandbox_policy_for_workspace(config: &CodexConfig, workspace: &WorkspaceConfig) -> Value {
    match config.sandbox.as_str() {
        "danger-full-access" => json!({ "type": "dangerFullAccess" }),
        "read-only" => json!({
            "type": "readOnly",
            "access": { "type": "cwdAndChildren" },
            "networkAccess": true,
        }),
        "workspace-write" => json!({
            "type": "workspaceWrite",
            "writableRoots": workspace
                .writable_roots
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
            "readOnlyAccess": { "type": "cwdAndChildren" },
            "networkAccess": "enabled",
            "excludeTmpdirEnvVar": false,
            "excludeSlashTmp": false,
        }),
        other => json!({
            "type": "workspaceWrite",
            "writableRoots": workspace
                .writable_roots
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
            "readOnlyAccess": { "type": "cwdAndChildren" },
            "networkAccess": "enabled",
            "excludeTmpdirEnvVar": false,
            "excludeSlashTmp": false,
            "fallbackSandbox": other,
        }),
    }
}

fn request_to_user_inputs(request: CodexRequest) -> Vec<Value> {
    let mut input = vec![json!({
        "type": "text",
        "text": request.prompt,
        "text_elements": [],
    })];
    for image_path in request.image_paths {
        input.push(json!({
            "type": "localImage",
            "path": image_path.display().to_string(),
        }));
    }
    input
}

fn turn_result_turn_id(result: &Value) -> Result<String> {
    result
        .get("turn")
        .and_then(|turn| turn.get("id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("turn/start result missing turn.id"))
}

fn thread_result_thread_id(result: &Value) -> Result<String> {
    result
        .get("thread")
        .and_then(|thread| thread.get("id"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("thread result missing thread.id"))
}

fn response_id(message: &Value) -> Option<String> {
    let id = message.get("id")?;
    match id {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_approval_request(message: &Value) -> Result<Option<CodexApprovalRequest>> {
    let request_id = match response_id(message) {
        Some(request_id) => request_id,
        None => return Ok(None),
    };
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("server request missing method"))?;
    let params = message
        .get("params")
        .cloned()
        .ok_or_else(|| anyhow!("server request missing params"))?;

    let (request_kind, summary_text) = match method {
        "item/commandExecution/requestApproval" | "execCommandApproval" => (
            ApprovalRequestKind::CommandExecution,
            summarize_command_approval(&params),
        ),
        "item/fileChange/requestApproval" | "applyPatchApproval" => (
            ApprovalRequestKind::FileChange,
            summarize_file_change_approval(&params),
        ),
        "item/permissions/requestApproval" => (
            ApprovalRequestKind::Permissions,
            summarize_permissions_approval(&params),
        ),
        "item/tool/requestUserInput" => (
            ApprovalRequestKind::ToolUserInput,
            summarize_tool_user_input(&params),
        ),
        _ => return Ok(None),
    };

    Ok(Some(CodexApprovalRequest {
        request_id,
        thread_id: required_param_str(&params, "threadId")?.to_owned(),
        turn_id: required_param_str(&params, "turnId")?.to_owned(),
        item_id: required_param_str(&params, "itemId")?.to_owned(),
        request_kind,
        summary_text,
        raw_payload_json: serde_json::to_string(&params)
            .context("failed to serialize approval request params")?,
    }))
}

fn approval_response(request: &ApprovalRequestRecord, approved: bool) -> Result<Value> {
    let raw_payload: Value =
        serde_json::from_str(&request.raw_payload_json).context("invalid approval payload JSON")?;
    let response = match request.request_kind {
        ApprovalRequestKind::CommandExecution => json!({
            "decision": if approved { "accept" } else { "decline" }
        }),
        ApprovalRequestKind::FileChange => json!({
            "decision": if approved { "accept" } else { "decline" }
        }),
        ApprovalRequestKind::Permissions => {
            if approved {
                let granted = raw_payload
                    .get("permissions")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                json!({
                    "permissions": granted,
                    "scope": "turn",
                })
            } else {
                json!({
                    "permissions": {},
                    "scope": "turn",
                })
            }
        }
        ApprovalRequestKind::ToolUserInput => {
            if approved {
                bail!("tool user input approvals are not implemented yet");
            }
            json!({ "answers": {} })
        }
    };
    Ok(response)
}

fn summarize_command_approval(params: &Value) -> String {
    let command = params
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("不明なコマンド");
    let cwd = params.get("cwd").and_then(Value::as_str);
    match cwd {
        Some(cwd) if !cwd.is_empty() => {
            format!("コマンド実行の承認待ち: `{command}`\n作業場所: `{cwd}`")
        }
        _ => format!("コマンド実行の承認待ち: `{command}`"),
    }
}

fn summarize_file_change_approval(params: &Value) -> String {
    if let Some(grant_root) = params.get("grantRoot").and_then(Value::as_str) {
        return format!("ファイル変更の承認待ち: `{grant_root}` への書き込み");
    }
    if let Some(reason) = params.get("reason").and_then(Value::as_str) {
        if !reason.trim().is_empty() {
            return format!("ファイル変更の承認待ち: {reason}");
        }
    }
    "ファイル変更の承認待ち".to_owned()
}

fn summarize_permissions_approval(params: &Value) -> String {
    let reason = params
        .get("reason")
        .and_then(Value::as_str)
        .filter(|reason| !reason.trim().is_empty())
        .unwrap_or("追加の権限が必要です。");
    format!("追加権限の承認待ち: {reason}")
}

fn summarize_tool_user_input(params: &Value) -> String {
    let count = params
        .get("questions")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    format!("追加の入力待ち: 質問数 {count}")
}

fn required_param_str<'a>(params: &'a Value, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("approval params missing `{key}`"))
}

fn turn_status_exit_code(status: &str) -> i32 {
    match status {
        "completed" => 0,
        "interrupted" => 130,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::ApprovalRequestKind;

    #[test]
    fn parses_command_approval_request() {
        let request = parse_approval_request(&json!({
            "id": "req-1",
            "method": "item/commandExecution/requestApproval",
            "params": {
                "threadId": "thread-1",
                "turnId": "turn-1",
                "itemId": "item-1",
                "command": "npm test",
                "cwd": "C:/workspace"
            }
        }))
        .expect("approval should parse")
        .expect("approval should exist");

        assert_eq!(request.request_kind, ApprovalRequestKind::CommandExecution);
        assert!(request.summary_text.contains("npm test"));
        assert!(request.summary_text.contains("C:/workspace"));
    }

    #[test]
    fn parses_permissions_approval_request() {
        let request = parse_approval_request(&json!({
            "id": "req-2",
            "method": "item/permissions/requestApproval",
            "params": {
                "threadId": "thread-1",
                "turnId": "turn-1",
                "itemId": "item-1",
                "reason": "network access required",
                "permissions": {
                    "network": { "kind": "full" },
                    "fileSystem": null
                }
            }
        }))
        .expect("approval should parse")
        .expect("approval should exist");

        assert_eq!(request.request_kind, ApprovalRequestKind::Permissions);
        assert!(request.summary_text.contains("network access required"));
    }

    #[test]
    fn builds_decline_response_for_command_execution() {
        let request = ApprovalRequestRecord {
            request_id: "req-1".to_owned(),
            lane_id: "lane-1".to_owned(),
            run_id: "run-1".to_owned(),
            thread_id: "thread-1".to_owned(),
            turn_id: "turn-1".to_owned(),
            item_id: "item-1".to_owned(),
            request_kind: ApprovalRequestKind::CommandExecution,
            summary_text: "command".to_owned(),
            raw_payload_json: "{}".to_owned(),
            status: crate::store::ApprovalRequestStatus::Pending,
            requested_at_ms: 0,
            resolved_at_ms: None,
            resolved_by_sender_id: None,
            telegram_message_id: None,
        };

        let response = approval_response(&request, false).expect("response should build");
        assert_eq!(response, json!({ "decision": "decline" }));
    }

    #[test]
    fn builds_accept_response_for_permissions_request() {
        let request = ApprovalRequestRecord {
            request_id: "req-2".to_owned(),
            lane_id: "lane-1".to_owned(),
            run_id: "run-1".to_owned(),
            thread_id: "thread-1".to_owned(),
            turn_id: "turn-1".to_owned(),
            item_id: "item-1".to_owned(),
            request_kind: ApprovalRequestKind::Permissions,
            summary_text: "permissions".to_owned(),
            raw_payload_json: json!({
                "permissions": {
                    "network": { "kind": "full" },
                    "fileSystem": null
                }
            })
            .to_string(),
            status: crate::store::ApprovalRequestStatus::Pending,
            requested_at_ms: 0,
            resolved_at_ms: None,
            resolved_by_sender_id: None,
            telegram_message_id: None,
        };

        let response = approval_response(&request, true).expect("response should build");
        assert_eq!(
            response,
            json!({
                "permissions": {
                    "network": { "kind": "full" },
                    "fileSystem": null
                },
                "scope": "turn",
            })
        );
    }
}
