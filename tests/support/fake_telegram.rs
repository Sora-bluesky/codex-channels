use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};

#[derive(Clone)]
struct FakeTelegramState {
    token: String,
    bot_id: i64,
    username: Option<String>,
    next_update_id: i64,
    next_message_id: i64,
    updates: Vec<Value>,
    sent_messages: Vec<SentMessageRecord>,
    edited_messages: Vec<EditedMessageRecord>,
    callback_answers: Vec<CallbackAnswerRecord>,
    webhook_url: String,
    delete_webhook_calls: Vec<DeleteWebhookCall>,
    files: HashMap<String, RegisteredFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentMessageRecord {
    pub chat_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditedMessageRecord {
    pub chat_id: i64,
    pub message_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackAnswerRecord {
    pub callback_query_id: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteWebhookCall {
    pub drop_pending_updates: bool,
}

#[derive(Debug, Clone)]
struct RegisteredFile {
    file_unique_id: String,
    file_path: String,
    bytes: Vec<u8>,
}

pub struct FakeTelegramServer {
    address: SocketAddr,
    state: Arc<Mutex<FakeTelegramState>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl FakeTelegramServer {
    pub async fn start(token: &str) -> Result<Self> {
        let state = Arc::new(Mutex::new(FakeTelegramState {
            token: token.to_owned(),
            bot_id: 77_000,
            username: Some("remotty_test_bot".to_owned()),
            next_update_id: 1,
            next_message_id: 1,
            updates: Vec::new(),
            sent_messages: Vec::new(),
            edited_messages: Vec::new(),
            callback_answers: Vec::new(),
            webhook_url: String::new(),
            delete_webhook_calls: Vec::new(),
            files: HashMap::new(),
        }));
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind fake telegram server")?;
        let address = listener
            .local_addr()
            .context("failed to read fake telegram address")?;
        let router = Router::new()
            .route("/{*path}", any(route_request))
            .with_state(state.clone());
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });
        Ok(Self {
            address,
            state,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub fn api_base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn file_base_url(&self) -> String {
        format!("http://{}/file", self.address)
    }

    pub async fn enqueue_message(&self, chat_id: i64, sender_id: i64, text: &str) -> Result<i64> {
        let mut state = self.state.lock().await;
        let update_id = state.next_update_id;
        state.next_update_id += 1;
        let message_id = state.next_message_id;
        state.next_message_id += 1;
        let sent_at_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();
        state.updates.push(json!({
            "update_id": update_id,
            "message": {
                "message_id": message_id,
                "date": sent_at_s,
                "text": text,
                "chat": {
                    "id": chat_id,
                    "type": "private"
                },
                "from": {
                    "id": sender_id
                }
            }
        }));
        Ok(update_id)
    }

    pub async fn enqueue_callback_query(
        &self,
        chat_id: i64,
        sender_id: i64,
        message_id: i64,
        data: &str,
    ) -> Result<i64> {
        let mut state = self.state.lock().await;
        let update_id = state.next_update_id;
        state.next_update_id += 1;
        state.updates.push(json!({
            "update_id": update_id,
            "callback_query": {
                "id": format!("callback-{update_id}"),
                "from": {
                    "id": sender_id
                },
                "data": data,
                "message": {
                    "message_id": message_id,
                    "chat": {
                        "id": chat_id,
                        "type": "private"
                    }
                }
            }
        }));
        Ok(update_id)
    }

    pub async fn set_webhook_url(&self, url: &str) {
        let mut state = self.state.lock().await;
        state.webhook_url = url.to_owned();
    }

    pub async fn register_file(
        &self,
        file_id: &str,
        file_unique_id: &str,
        file_path: &str,
        bytes: &[u8],
    ) {
        let mut state = self.state.lock().await;
        state.files.insert(
            file_id.to_owned(),
            RegisteredFile {
                file_unique_id: file_unique_id.to_owned(),
                file_path: file_path.to_owned(),
                bytes: bytes.to_vec(),
            },
        );
    }

    pub async fn sent_messages(&self) -> Vec<SentMessageRecord> {
        self.state.lock().await.sent_messages.clone()
    }

    pub async fn edited_messages(&self) -> Vec<EditedMessageRecord> {
        self.state.lock().await.edited_messages.clone()
    }

    pub async fn callback_answers(&self) -> Vec<CallbackAnswerRecord> {
        self.state.lock().await.callback_answers.clone()
    }

    pub async fn delete_webhook_calls(&self) -> Vec<DeleteWebhookCall> {
        self.state.lock().await.delete_webhook_calls.clone()
    }
}

impl Drop for FakeTelegramServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
    }
}

async fn route_request(
    State(state): State<Arc<Mutex<FakeTelegramState>>>,
    method: Method,
    uri: axum::http::Uri,
    body: Bytes,
) -> Response {
    match route_request_inner(state, method, uri.path(), body).await {
        Ok(response) => response,
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn route_request_inner(
    state: Arc<Mutex<FakeTelegramState>>,
    method: Method,
    path: &str,
    body: Bytes,
) -> Result<Response> {
    let path = path.trim_start_matches('/');
    if let Some(path) = path.strip_prefix("file/bot") {
        let (token, file_path) = path
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("invalid fake telegram file path"))?;
        let state_guard = state.lock().await;
        if token != state_guard.token {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
        let registered = state_guard
            .files
            .values()
            .find(|file| file.file_path == file_path)
            .ok_or_else(|| anyhow::anyhow!("unknown fake telegram file"))?;
        return Ok((StatusCode::OK, registered.bytes.clone()).into_response());
    }

    let path = path
        .strip_prefix("bot")
        .ok_or_else(|| anyhow::anyhow!("invalid fake telegram path"))?;
    let (token, method_name) = path
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("invalid fake telegram method path"))?;
    let payload = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice::<Value>(&body).context("failed to decode fake telegram body")?
    };

    let mut state_guard = state.lock().await;
    if token != state_guard.token {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    match (method, method_name) {
        (Method::GET, "getMe") | (Method::POST, "getMe") => Ok(Json(json!({
            "ok": true,
            "result": {
                "id": state_guard.bot_id,
                "username": state_guard.username,
            }
        }))
        .into_response()),
        (Method::POST, "getUpdates") => {
            let offset = payload.get("offset").and_then(Value::as_i64).unwrap_or(0);
            let updates = state_guard
                .updates
                .iter()
                .filter(|update| {
                    update.get("update_id").and_then(Value::as_i64).unwrap_or(0) >= offset
                })
                .cloned()
                .collect::<Vec<_>>();
            Ok(Json(json!({
                "ok": true,
                "result": updates,
            }))
            .into_response())
        }
        (Method::POST, "sendMessage") => {
            let chat_id = payload
                .get("chat_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing sendMessage chat_id"))?;
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing sendMessage text"))?;
            let message_id = state_guard.next_message_id;
            state_guard.next_message_id += 1;
            state_guard.sent_messages.push(SentMessageRecord {
                chat_id,
                text: text.to_owned(),
            });
            Ok(Json(json!({
                "ok": true,
                "result": {
                    "message_id": message_id,
                }
            }))
            .into_response())
        }
        (Method::POST, "editMessageText") => {
            let chat_id = payload
                .get("chat_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing editMessageText chat_id"))?;
            let message_id = payload
                .get("message_id")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow::anyhow!("missing editMessageText message_id"))?;
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing editMessageText text"))?;
            state_guard.edited_messages.push(EditedMessageRecord {
                chat_id,
                message_id,
                text: text.to_owned(),
            });
            Ok(Json(json!({ "ok": true, "result": true })).into_response())
        }
        (Method::POST, "answerCallbackQuery") => {
            let callback_query_id = payload
                .get("callback_query_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing answerCallbackQuery id"))?;
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            state_guard.callback_answers.push(CallbackAnswerRecord {
                callback_query_id: callback_query_id.to_owned(),
                text,
            });
            Ok(Json(json!({ "ok": true, "result": true })).into_response())
        }
        (Method::POST, "getWebhookInfo") => Ok(Json(json!({
            "ok": true,
            "result": {
                "url": state_guard.webhook_url,
            }
        }))
        .into_response()),
        (Method::POST, "deleteWebhook") => {
            let drop_pending_updates = payload
                .get("drop_pending_updates")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            state_guard.delete_webhook_calls.push(DeleteWebhookCall {
                drop_pending_updates,
            });
            state_guard.webhook_url.clear();
            if drop_pending_updates {
                state_guard.updates.clear();
            }
            Ok(Json(json!({ "ok": true, "result": true })).into_response())
        }
        (Method::POST, "getFile") => {
            let file_id = payload
                .get("file_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing getFile file_id"))?;
            let registered = state_guard
                .files
                .get(file_id)
                .ok_or_else(|| anyhow::anyhow!("unknown file id"))?;
            Ok(Json(json!({
                "ok": true,
                "result": {
                    "file_id": file_id,
                    "file_unique_id": registered.file_unique_id,
                    "file_size": registered.bytes.len(),
                    "file_path": registered.file_path,
                }
            }))
            .into_response())
        }
        _ => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}
