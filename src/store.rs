use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params, types::Type};
use uuid::Uuid;

use crate::config::LaneMode;

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneState {
    Running,
    WaitingReply,
    Idle,
    NeedsLocalApproval,
    Failed,
}

impl LaneState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::WaitingReply => "waiting_reply",
            Self::Idle => "idle",
            Self::NeedsLocalApproval => "needs_local_approval",
            Self::Failed => "failed",
        }
    }

    fn from_str(value: &str) -> std::result::Result<Self, String> {
        match value {
            "running" => Ok(Self::Running),
            "waiting_reply" => Ok(Self::WaitingReply),
            "idle" => Ok(Self::Idle),
            "needs_local_approval" => Ok(Self::NeedsLocalApproval),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown lane state: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizedSender {
    pub sender_id: i64,
    pub platform: String,
    pub display_name: Option<String>,
    pub status: String,
    pub approved_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct LaneRecord {
    pub lane_id: String,
    pub chat_id: i64,
    pub thread_key: String,
    pub workspace_id: String,
    pub mode: LaneMode,
    pub state: LaneState,
    pub codex_session_id: Option<String>,
    pub extra_turn_budget: i64,
    pub waiting_since_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NewRun {
    pub lane_id: String,
    pub run_kind: String,
}

#[derive(Debug, Clone)]
pub struct RunRecord {
    pub run_id: String,
    pub lane_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalRequestKind {
    CommandExecution,
    FileChange,
    Permissions,
    ToolUserInput,
}

impl ApprovalRequestKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::CommandExecution => "command_execution",
            Self::FileChange => "file_change",
            Self::Permissions => "permissions",
            Self::ToolUserInput => "tool_user_input",
        }
    }

    fn from_str(value: &str) -> std::result::Result<Self, String> {
        match value {
            "command_execution" => Ok(Self::CommandExecution),
            "file_change" => Ok(Self::FileChange),
            "permissions" => Ok(Self::Permissions),
            "tool_user_input" => Ok(Self::ToolUserInput),
            other => Err(format!("unknown approval request kind: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalRequestStatus {
    Pending,
    Approved,
    Declined,
    TimedOut,
}

impl ApprovalRequestStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Declined => "declined",
            Self::TimedOut => "timed_out",
        }
    }

    fn from_str(value: &str) -> std::result::Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "declined" => Ok(Self::Declined),
            "timed_out" => Ok(Self::TimedOut),
            other => Err(format!("unknown approval request status: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApprovalRequestRecord {
    pub request_id: String,
    pub lane_id: String,
    pub run_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub request_kind: ApprovalRequestKind,
    pub summary_text: String,
    pub raw_payload_json: String,
    pub status: ApprovalRequestStatus,
    pub requested_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
    pub resolved_by_sender_id: Option<i64>,
    pub telegram_message_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NewApprovalRequest {
    pub request_id: String,
    pub lane_id: String,
    pub run_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub request_kind: ApprovalRequestKind,
    pub summary_text: String,
    pub raw_payload_json: String,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).context("failed to open sqlite database")?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS authorized_senders (
                sender_id INTEGER PRIMARY KEY,
                platform TEXT NOT NULL,
                display_name TEXT,
                status TEXT NOT NULL,
                approved_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS lanes (
                lane_id TEXT PRIMARY KEY,
                chat_id INTEGER NOT NULL,
                thread_key TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                mode TEXT NOT NULL,
                state TEXT NOT NULL,
                codex_session_id TEXT,
                extra_turn_budget INTEGER NOT NULL DEFAULT 0,
                waiting_since_ms INTEGER,
                UNIQUE(chat_id, thread_key)
            );

            CREATE TABLE IF NOT EXISTS runs (
                run_id TEXT PRIMARY KEY,
                lane_id TEXT NOT NULL,
                run_kind TEXT NOT NULL,
                started_at_ms INTEGER NOT NULL,
                ended_at_ms INTEGER,
                exit_code INTEGER,
                completion_reason TEXT,
                approval_pending INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS telegram_updates (
                update_id INTEGER PRIMARY KEY,
                chat_id INTEGER NOT NULL,
                sender_id INTEGER,
                update_kind TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                lane_id TEXT NOT NULL,
                run_id TEXT,
                direction TEXT NOT NULL,
                message_kind TEXT NOT NULL,
                telegram_message_id INTEGER,
                body_text TEXT,
                payload_json TEXT
            );

            INSERT OR IGNORE INTO schema_migrations(version, applied_at_ms)
                VALUES (1, unixepoch('subsec') * 1000);
        "#;
        self.with_conn(|conn| conn.execute_batch(sql))?;
        self.with_conn(|conn| {
            ensure_column_exists(
                conn,
                "runs",
                "approval_request_count",
                "ALTER TABLE runs ADD COLUMN approval_request_count INTEGER NOT NULL DEFAULT 0",
            )?;
            ensure_column_exists(
                conn,
                "runs",
                "approval_resolved_count",
                "ALTER TABLE runs ADD COLUMN approval_resolved_count INTEGER NOT NULL DEFAULT 0",
            )?;
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS approval_requests (
                    request_id TEXT PRIMARY KEY,
                    lane_id TEXT NOT NULL,
                    run_id TEXT NOT NULL,
                    thread_id TEXT NOT NULL,
                    turn_id TEXT NOT NULL,
                    item_id TEXT NOT NULL,
                    request_kind TEXT NOT NULL,
                    summary_text TEXT NOT NULL,
                    raw_payload_json TEXT NOT NULL,
                    status TEXT NOT NULL,
                    requested_at_ms INTEGER NOT NULL,
                    resolved_at_ms INTEGER,
                    resolved_by_sender_id INTEGER,
                    telegram_message_id INTEGER
                );
                "#,
            )
        })?;
        Ok(())
    }

    pub fn upsert_authorized_sender(&self, sender: AuthorizedSender) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO authorized_senders(sender_id, platform, display_name, status, approved_at_ms)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(sender_id) DO UPDATE SET
                    platform = excluded.platform,
                    display_name = excluded.display_name,
                    status = excluded.status,
                    approved_at_ms = excluded.approved_at_ms
                "#,
                params![
                    sender.sender_id,
                    sender.platform,
                    sender.display_name,
                    sender.status,
                    sender.approved_at_ms,
                ],
            )
        })?;
        Ok(())
    }

    pub fn is_authorized_sender(&self, sender_id: i64) -> Result<bool> {
        let found: Option<i64> = self.with_conn(|conn| {
            conn.query_row(
                "SELECT sender_id FROM authorized_senders WHERE sender_id = ?1 AND status = 'active'",
                params![sender_id],
                |row| row.get(0),
            )
            .optional()
        })?;
        Ok(found.is_some())
    }

    pub fn insert_seen_update(
        &self,
        update_id: i64,
        chat_id: i64,
        sender_id: Option<i64>,
        update_kind: &str,
        payload_json: &str,
    ) -> Result<bool> {
        let inserted = self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT OR IGNORE INTO telegram_updates(update_id, chat_id, sender_id, update_kind, payload_json)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![update_id, chat_id, sender_id, update_kind, payload_json],
            )
        })?;
        Ok(inserted > 0)
    }

    pub fn get_or_create_lane(
        &self,
        chat_id: i64,
        thread_key: &str,
        workspace_id: &str,
        mode: LaneMode,
        extra_turn_budget: i64,
    ) -> Result<LaneRecord> {
        if let Some(lane) = self.find_lane(chat_id, thread_key)? {
            return Ok(lane);
        }
        let lane = LaneRecord {
            lane_id: Uuid::new_v4().to_string(),
            chat_id,
            thread_key: thread_key.to_owned(),
            workspace_id: workspace_id.to_owned(),
            mode,
            state: LaneState::Idle,
            codex_session_id: None,
            extra_turn_budget,
            waiting_since_ms: None,
        };
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO lanes(
                    lane_id, chat_id, thread_key, workspace_id, mode, state,
                    codex_session_id, extra_turn_budget, waiting_since_ms
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    lane.lane_id,
                    lane.chat_id,
                    lane.thread_key,
                    lane.workspace_id,
                    mode_to_str(lane.mode),
                    lane.state.as_str(),
                    lane.codex_session_id,
                    lane.extra_turn_budget,
                    lane.waiting_since_ms,
                ],
            )
        })?;
        Ok(lane)
    }

    pub fn find_lane(&self, chat_id: i64, thread_key: &str) -> Result<Option<LaneRecord>> {
        self.with_conn(|conn| {
            conn.query_row(
                r#"
                SELECT lane_id, chat_id, thread_key, workspace_id, mode, state,
                       codex_session_id, extra_turn_budget, waiting_since_ms
                FROM lanes
                WHERE chat_id = ?1 AND thread_key = ?2
                "#,
                params![chat_id, thread_key],
                |row| {
                    Ok(LaneRecord {
                        lane_id: row.get(0)?,
                        chat_id: row.get(1)?,
                        thread_key: row.get(2)?,
                        workspace_id: row.get(3)?,
                        mode: mode_from_str(&row.get::<_, String>(4)?).map_err(|err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                Type::Text,
                                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                            )
                        })?,
                        state: LaneState::from_str(&row.get::<_, String>(5)?).map_err(|err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                5,
                                Type::Text,
                                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                            )
                        })?,
                        codex_session_id: row.get(6)?,
                        extra_turn_budget: row.get(7)?,
                        waiting_since_ms: row.get(8)?,
                    })
                },
            )
            .optional()
        })
    }

    pub fn update_lane_state(
        &self,
        lane_id: &str,
        state: LaneState,
        codex_session_id: Option<&str>,
    ) -> Result<()> {
        let waiting_since_ms = if state == LaneState::WaitingReply {
            Some(Utc::now().timestamp_millis())
        } else {
            None
        };
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE lanes
                SET state = ?2,
                    codex_session_id = COALESCE(?3, codex_session_id),
                    waiting_since_ms = ?4
                WHERE lane_id = ?1
                "#,
                params![lane_id, state.as_str(), codex_session_id, waiting_since_ms],
            )
        })?;
        Ok(())
    }

    pub fn clear_lane_session(&self, lane_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE lanes
                SET state = 'idle',
                    codex_session_id = NULL,
                    waiting_since_ms = NULL
                WHERE lane_id = ?1
                "#,
                params![lane_id],
            )
        })?;
        Ok(())
    }

    pub fn update_lane_mode(
        &self,
        lane_id: &str,
        mode: LaneMode,
        extra_turn_budget: i64,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE lanes
                SET mode = ?2,
                    extra_turn_budget = ?3
                WHERE lane_id = ?1
                "#,
                params![lane_id, mode_to_str(mode), extra_turn_budget],
            )
        })?;
        Ok(())
    }

    pub fn update_lane_workspace(&self, lane_id: &str, workspace_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE lanes
                SET workspace_id = ?2,
                    state = 'idle',
                    codex_session_id = NULL,
                    waiting_since_ms = NULL
                WHERE lane_id = ?1
                "#,
                params![lane_id, workspace_id],
            )
        })?;
        Ok(())
    }

    pub fn insert_run(&self, new_run: NewRun) -> Result<RunRecord> {
        let run = RunRecord {
            run_id: Uuid::new_v4().to_string(),
            lane_id: new_run.lane_id,
        };
        let now = Utc::now().timestamp_millis();
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO runs(run_id, lane_id, run_kind, started_at_ms)
                VALUES (?1, ?2, ?3, ?4)
                "#,
                params![run.run_id, run.lane_id, new_run.run_kind, now],
            )
        })?;
        Ok(run)
    }

    pub fn finish_run(
        &self,
        run_id: &str,
        exit_code: Option<i32>,
        completion_reason: &str,
        approval_pending: bool,
        approval_request_count: i64,
        approval_resolved_count: i64,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE runs
                SET ended_at_ms = ?2,
                    exit_code = ?3,
                    completion_reason = ?4,
                    approval_pending = ?5,
                    approval_request_count = ?6,
                    approval_resolved_count = ?7
                WHERE run_id = ?1
                "#,
                params![
                    run_id,
                    now,
                    exit_code,
                    completion_reason,
                    approval_pending as i32,
                    approval_request_count,
                    approval_resolved_count,
                ],
            )
        })?;
        Ok(())
    }

    pub fn insert_approval_request(&self, request: NewApprovalRequest) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO approval_requests(
                    request_id, lane_id, run_id, thread_id, turn_id, item_id,
                    request_kind, summary_text, raw_payload_json, status, requested_at_ms
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', ?10)
                ON CONFLICT(request_id) DO NOTHING
                "#,
                params![
                    request.request_id,
                    request.lane_id,
                    request.run_id,
                    request.thread_id,
                    request.turn_id,
                    request.item_id,
                    request.request_kind.as_str(),
                    request.summary_text,
                    request.raw_payload_json,
                    now,
                ],
            )
        })?;
        Ok(())
    }

    pub fn find_approval_request(&self, request_id: &str) -> Result<Option<ApprovalRequestRecord>> {
        self.with_conn(|conn| {
            conn.query_row(
                r#"
                SELECT request_id, lane_id, run_id, thread_id, turn_id, item_id,
                       request_kind, summary_text, raw_payload_json, status,
                       requested_at_ms, resolved_at_ms, resolved_by_sender_id, telegram_message_id
                FROM approval_requests
                WHERE request_id = ?1
                "#,
                params![request_id],
                |row| {
                    Ok(ApprovalRequestRecord {
                        request_id: row.get(0)?,
                        lane_id: row.get(1)?,
                        run_id: row.get(2)?,
                        thread_id: row.get(3)?,
                        turn_id: row.get(4)?,
                        item_id: row.get(5)?,
                        request_kind: ApprovalRequestKind::from_str(&row.get::<_, String>(6)?)
                            .map_err(|err| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    6,
                                    Type::Text,
                                    Box::new(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        err,
                                    )),
                                )
                            })?,
                        summary_text: row.get(7)?,
                        raw_payload_json: row.get(8)?,
                        status: ApprovalRequestStatus::from_str(&row.get::<_, String>(9)?)
                            .map_err(|err| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    9,
                                    Type::Text,
                                    Box::new(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        err,
                                    )),
                                )
                            })?,
                        requested_at_ms: row.get(10)?,
                        resolved_at_ms: row.get(11)?,
                        resolved_by_sender_id: row.get(12)?,
                        telegram_message_id: row.get(13)?,
                    })
                },
            )
            .optional()
        })
    }

    pub fn resolve_approval_request(
        &self,
        request_id: &str,
        status: ApprovalRequestStatus,
        resolved_by_sender_id: i64,
    ) -> Result<bool> {
        let now = Utc::now().timestamp_millis();
        let updated = self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE approval_requests
                SET status = ?2,
                    resolved_at_ms = ?3,
                    resolved_by_sender_id = ?4
                WHERE request_id = ?1 AND status = 'pending'
                "#,
                params![request_id, status.as_str(), now, resolved_by_sender_id],
            )
        })?;
        Ok(updated > 0)
    }

    pub fn set_approval_request_message_id(
        &self,
        request_id: &str,
        telegram_message_id: i64,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE approval_requests SET telegram_message_id = ?2 WHERE request_id = ?1",
                params![request_id, telegram_message_id],
            )
        })?;
        Ok(())
    }

    pub fn list_pending_approval_requests_for_lane(
        &self,
        lane_id: &str,
    ) -> Result<Vec<ApprovalRequestRecord>> {
        self.with_conn(|conn| {
            let mut statement = conn.prepare(
                r#"
                SELECT request_id, lane_id, run_id, thread_id, turn_id, item_id,
                       request_kind, summary_text, raw_payload_json, status,
                       requested_at_ms, resolved_at_ms, resolved_by_sender_id, telegram_message_id
                FROM approval_requests
                WHERE lane_id = ?1 AND status = 'pending'
                ORDER BY requested_at_ms ASC
                "#,
            )?;
            let rows = statement.query_map(params![lane_id], |row| {
                Ok(ApprovalRequestRecord {
                    request_id: row.get(0)?,
                    lane_id: row.get(1)?,
                    run_id: row.get(2)?,
                    thread_id: row.get(3)?,
                    turn_id: row.get(4)?,
                    item_id: row.get(5)?,
                    request_kind: ApprovalRequestKind::from_str(&row.get::<_, String>(6)?)
                        .map_err(|err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                6,
                                Type::Text,
                                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                            )
                        })?,
                    summary_text: row.get(7)?,
                    raw_payload_json: row.get(8)?,
                    status: ApprovalRequestStatus::from_str(&row.get::<_, String>(9)?).map_err(
                        |err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                9,
                                Type::Text,
                                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                            )
                        },
                    )?,
                    requested_at_ms: row.get(10)?,
                    resolved_at_ms: row.get(11)?,
                    resolved_by_sender_id: row.get(12)?,
                    telegram_message_id: row.get(13)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
    }

    pub fn insert_message(
        &self,
        lane_id: &str,
        run_id: Option<&str>,
        direction: &str,
        message_kind: &str,
        telegram_message_id: Option<i64>,
        body_text: Option<&str>,
        payload_json: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO messages(
                    lane_id, run_id, direction, message_kind, telegram_message_id, body_text, payload_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    lane_id,
                    run_id,
                    direction,
                    message_kind,
                    telegram_message_id,
                    body_text,
                    payload_json,
                ],
            )
        })?;
        Ok(())
    }

    fn with_conn<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T>,
    {
        let guard = self
            .conn
            .lock()
            .map_err(|_| anyhow!("sqlite mutex poisoned"))?;
        f(&guard).context("sqlite operation failed")
    }
}

fn ensure_column_exists(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    alter_sql: &str,
) -> rusqlite::Result<()> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let exists = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .any(|existing| existing == column_name);
    if !exists {
        conn.execute_batch(alter_sql)?;
    }
    Ok(())
}

fn mode_to_str(mode: LaneMode) -> &'static str {
    match mode {
        LaneMode::AwaitReply => "await_reply",
        LaneMode::Infinite => "infinite",
        LaneMode::CompletionChecks => "completion_checks",
        LaneMode::MaxTurns => "max_turns",
    }
}

fn mode_from_str(value: &str) -> std::result::Result<LaneMode, String> {
    match value {
        "await_reply" => Ok(LaneMode::AwaitReply),
        "infinite" => Ok(LaneMode::Infinite),
        "completion_checks" => Ok(LaneMode::CompletionChecks),
        "max_turns" => Ok(LaneMode::MaxTurns),
        other => Err(format!("unknown lane mode: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::{TempDir, tempdir};

    fn temp_store() -> (TempDir, Store) {
        let dir = tempdir().expect("temp dir");
        let store = Store::open(dir.path().join("store.db")).expect("store");
        (dir, store)
    }

    #[test]
    fn find_lane_returns_lane_for_chat_and_thread() {
        let (_dir, store) = temp_store();
        let created = store
            .get_or_create_lane(42, "555", "workspace", LaneMode::AwaitReply, 0)
            .expect("lane");

        let fetched = store
            .find_lane(42, "555")
            .expect("query")
            .expect("lane exists");

        assert_eq!(fetched.lane_id, created.lane_id);
        assert_eq!(fetched.chat_id, 42);
        assert_eq!(fetched.thread_key, "555");
    }

    #[test]
    fn clear_lane_session_resets_session_fields_and_state() {
        let (_dir, store) = temp_store();
        let lane = store
            .get_or_create_lane(42, "555", "workspace", LaneMode::MaxTurns, 2)
            .expect("lane");
        store
            .update_lane_state(&lane.lane_id, LaneState::WaitingReply, Some("session-1"))
            .expect("state update");
        store
            .with_conn(|conn| {
                conn.execute(
                    "UPDATE lanes SET extra_turn_budget = 2 WHERE lane_id = ?1",
                    params![&lane.lane_id],
                )
            })
            .expect("budget update");

        store
            .clear_lane_session(&lane.lane_id)
            .expect("session clear");

        let lane = store
            .find_lane(42, "555")
            .expect("query")
            .expect("lane exists");
        assert_eq!(lane.state, LaneState::Idle);
        assert_eq!(lane.codex_session_id, None);
        assert_eq!(lane.extra_turn_budget, 2);
        assert_eq!(lane.waiting_since_ms, None);
        assert_eq!(lane.mode, LaneMode::MaxTurns);
    }

    #[test]
    fn update_lane_mode_changes_mode_and_budget() {
        let (_dir, store) = temp_store();
        let lane = store
            .get_or_create_lane(42, "555", "workspace", LaneMode::AwaitReply, 0)
            .expect("lane");
        store
            .update_lane_state(&lane.lane_id, LaneState::WaitingReply, Some("session-1"))
            .expect("state update");

        store
            .update_lane_mode(&lane.lane_id, LaneMode::MaxTurns, 5)
            .expect("mode update");

        let lane = store
            .find_lane(42, "555")
            .expect("query")
            .expect("lane exists");
        assert_eq!(lane.mode, LaneMode::MaxTurns);
        assert_eq!(lane.extra_turn_budget, 5);
        assert_eq!(lane.state, LaneState::WaitingReply);
        assert_eq!(lane.codex_session_id.as_deref(), Some("session-1"));
        assert!(lane.waiting_since_ms.is_some());
    }

    #[test]
    fn create_lane_uses_requested_budget_for_max_turns() {
        let (_dir, store) = temp_store();

        let lane = store
            .get_or_create_lane(42, "555", "workspace", LaneMode::MaxTurns, 4)
            .expect("lane");

        assert_eq!(lane.extra_turn_budget, 4);
    }

    #[test]
    fn update_lane_workspace_changes_workspace_and_clears_session() {
        let (_dir, store) = temp_store();
        let lane = store
            .get_or_create_lane(42, "555", "main", LaneMode::AwaitReply, 0)
            .expect("lane");
        store
            .update_lane_state(&lane.lane_id, LaneState::WaitingReply, Some("session-1"))
            .expect("state update");

        store
            .update_lane_workspace(&lane.lane_id, "docs")
            .expect("workspace update");

        let lane = store
            .find_lane(42, "555")
            .expect("query")
            .expect("lane exists");
        assert_eq!(lane.workspace_id, "docs");
        assert_eq!(lane.state, LaneState::Idle);
        assert_eq!(lane.codex_session_id, None);
        assert_eq!(lane.waiting_since_ms, None);
    }

    #[test]
    fn update_lane_state_to_needs_local_approval_keeps_session_and_clears_waiting() {
        let (_dir, store) = temp_store();
        let lane = store
            .get_or_create_lane(42, "555", "main", LaneMode::AwaitReply, 0)
            .expect("lane");
        store
            .update_lane_state(&lane.lane_id, LaneState::WaitingReply, Some("session-1"))
            .expect("state update");

        store
            .update_lane_state(
                &lane.lane_id,
                LaneState::NeedsLocalApproval,
                Some("session-1"),
            )
            .expect("approval state update");

        let lane = store
            .find_lane(42, "555")
            .expect("query")
            .expect("lane exists");
        assert_eq!(lane.state, LaneState::NeedsLocalApproval);
        assert_eq!(lane.codex_session_id.as_deref(), Some("session-1"));
        assert_eq!(lane.waiting_since_ms, None);
    }

    #[test]
    fn finish_run_persists_approval_counts() {
        let (_dir, store) = temp_store();
        let run = store
            .insert_run(NewRun {
                lane_id: "lane-1".to_owned(),
                run_kind: "start".to_owned(),
            })
            .expect("run");

        store
            .finish_run(&run.run_id, None, "needs_local_approval", true, 2, 1)
            .expect("finish run");

        let (approval_pending, request_count, resolved_count): (i64, i64, i64) = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT approval_pending, approval_request_count, approval_resolved_count FROM runs WHERE run_id = ?1",
                    params![&run.run_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
            })
            .expect("read run");
        assert_eq!(approval_pending, 1);
        assert_eq!(request_count, 2);
        assert_eq!(resolved_count, 1);
    }

    #[test]
    fn approval_request_round_trip_supports_resolve_and_message_tracking() {
        let (_dir, store) = temp_store();
        store
            .insert_approval_request(NewApprovalRequest {
                request_id: "req-1".to_owned(),
                lane_id: "lane-1".to_owned(),
                run_id: "run-1".to_owned(),
                thread_id: "thread-1".to_owned(),
                turn_id: "turn-1".to_owned(),
                item_id: "item-1".to_owned(),
                request_kind: ApprovalRequestKind::CommandExecution,
                summary_text: "command approval".to_owned(),
                raw_payload_json: "{\"kind\":\"command\"}".to_owned(),
            })
            .expect("insert approval request");

        let pending = store
            .list_pending_approval_requests_for_lane("lane-1")
            .expect("pending approvals");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, ApprovalRequestStatus::Pending);

        store
            .set_approval_request_message_id("req-1", 55)
            .expect("set message id");
        let updated = store
            .resolve_approval_request("req-1", ApprovalRequestStatus::Approved, 99)
            .expect("resolve approval request");
        assert!(updated);

        let request = store
            .find_approval_request("req-1")
            .expect("find approval request")
            .expect("request exists");
        assert_eq!(request.status, ApprovalRequestStatus::Approved);
        assert_eq!(request.telegram_message_id, Some(55));
        assert_eq!(request.resolved_by_sender_id, Some(99));
        assert!(request.resolved_at_ms.is_some());

        let second_update = store
            .resolve_approval_request("req-1", ApprovalRequestStatus::Declined, 99)
            .expect("second resolve");
        assert!(!second_update);
    }
}
