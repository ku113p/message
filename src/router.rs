use std::fmt::Debug;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Json, Router};
use axum::routing::{get, post};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use sqlx::types::Uuid;
use tracing::{error, info, warn};
use crate::db::{self, create_topic, get_topic, create_message, get_messages, Message};

fn check_auth(headers: &HeaderMap) -> Result<(), StatusCode> {
    let token = match std::env::var("AUTH_TOKEN") {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };
    let raw = headers.get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let provided = raw.strip_prefix("Bearer ").unwrap_or(raw);
    if token != provided { return Err(StatusCode::UNAUTHORIZED); }
    Ok(())
}

pub async fn get_router(db: PgPool) -> Router {
    Router::new()
        .route("/topics", get(list_topics_handler))
        .route("/topics", post(create_topic_handler))
        .route("/topics/:topic_id/messages", post(create_message_handler))
        .route("/topics/:topic_id/messages", get(get_messages_handler))
        .with_state(db)
}

#[derive(Deserialize)]
struct CreateTopicRequest {
    name: String,
    tg_api: Option<TgApi>,
}

#[derive(Serialize)]
struct CreateTopicResponse {
    id: Uuid,
}

#[derive(Serialize)]
struct TopicListItem {
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
struct CreateMessageRequest {
    contacts: Value,
    text: String,
}

async fn create_topic_handler(
    State(db): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateTopicRequest>,
) -> Result<Json<CreateTopicResponse>, StatusCode> {
    check_auth(&headers)?;
    let tg_api = match payload.tg_api {
        None => None,
        Some(tg_api) => match tg_api.check().await {
            Ok(v) if v => serde_json::to_value(tg_api.clone()).map_or(None, |v| Some(v)),
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    };

    let topic = create_topic(&db, &payload.name, tg_api)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreateTopicResponse { id: topic.id }))
}

async fn create_message_handler(
    Path(topic_id): Path<Uuid>,
    State(db): State<PgPool>,
    Json(payload): Json<CreateMessageRequest>,
) -> Result<StatusCode, StatusCode> {
    match get_topic(&db, &topic_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)? {
        None => Err(StatusCode::NOT_FOUND),
        Some(topic) => {
            let message = create_message(&db, &payload.contacts, &payload.text, &topic_id)
                .await
                .map_err(|err| log_and_raise("Failed create_message", err))?;

            if let Some(tg_api) = topic.tg_api {
                match TgApi::try_from(tg_api) {
                    Err(_) => warn!("Failed parse TgApi for Topic(id={})", &topic_id),
                    Ok(tg_api) => tg_api.send(&topic.name, message).await
                };
            }

            Ok(StatusCode::CREATED)
        }
    }
}

fn log_and_raise(pre_message: &str, err: impl Debug) -> StatusCode {
    error!("{}: {:?}", pre_message, err);
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn get_messages_handler(
    Path(topic_id): Path<Uuid>,
    headers: HeaderMap,
    State(db): State<PgPool>,
) -> Result<Json<Vec<Message>>, StatusCode> {
    check_auth(&headers)?;

    let messages = get_messages(&db, &topic_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(messages))
}

async fn list_topics_handler(
    headers: HeaderMap,
    State(db): State<PgPool>,
) -> Result<Json<Vec<TopicListItem>>, StatusCode> {
    check_auth(&headers)?;

    let topics = db::list_topics(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items: Vec<TopicListItem> = topics.into_iter().map(|t| TopicListItem {
        id: t.id,
        name: t.name,
    }).collect();

    Ok(Json(items))
}

const TELEGRAM_API_URL: &str = "https://api.telegram.org/bot";

#[derive(Clone, Deserialize, Serialize)]
struct TgApi {
    api_key: String,
    chat_id: String,
}

impl TryFrom<Value> for TgApi {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value::<TgApi>(value)
            .map_err(|_| ())
    }
}

impl TgApi {
    async fn check(&self) -> Result<bool, String> {
        Client::new()
            .get(format!("{}{}/getMe", TELEGRAM_API_URL, self.api_key))
            .send()
            .await
            .map(|r| r.status().is_success())
            .map_err(|err| format!("Failed check api_key: {:?}", err))
    }

    async fn send(&self, topic_name: &str, message: Message) {
        let api_key = self.api_key.clone();
        let chat_id = self.chat_id.clone();
        let message_id = message.id;
        let contacts = serde_json::to_string(&message.contacts).unwrap_or_default();
        let message = format!(
            "Topic: {}\nText: {}\nContacts: {}",
            topic_name, message.text, contacts
        );

        tokio::spawn(async move {
            match Client::new()
                .post(format!("{}{}/sendMessage", TELEGRAM_API_URL, api_key))
                .json(&json!({"chat_id": chat_id, "text": message}))
                .send()
                .await {
                Err(err) => error!(
                    "Message(id={}) sending failed. Failed send request: {:?}",
                    message_id, err
                ),
                Ok(response) => match response.status().is_success() {
                    true => info!("Message(id={}) sent successfully", message_id),
                    false => match response.text().await {
                        Err(err) => error!(
                            "Message(id={}) sending failed. Failed get response: {:?}",
                            message_id, err
                        ),
                        Ok(text) => warn!(
                            "Message(id={}) sending failed. Response text={:?}",
                            message_id, text
                        )
                    }
                }
            }
        });
    }
}
