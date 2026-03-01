use serde::Serialize;
use sqlx::PgPool;
use sqlx::types::{Uuid, chrono::NaiveDateTime};

#[derive(Serialize, sqlx::FromRow)]
pub struct Topic {
    pub id: Uuid,
    pub name: String,
    pub tg_api: Option<serde_json::Value>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub contacts: serde_json::Value,
    pub text: String,
    pub topic_id: Uuid,
}

pub async fn create_topic(db: &PgPool, name: &str, tg_api: Option<serde_json::Value>) -> Result<Topic, sqlx::Error> {
    sqlx::query_as!(
        Topic,
        "INSERT INTO topic (name, tg_api) VALUES ($1, $2::jsonb) RETURNING *",
        name,
        tg_api,
    )
    .fetch_one(db)
    .await
}

pub async fn list_topics(db: &PgPool) -> Result<Vec<Topic>, sqlx::Error> {
    sqlx::query_as!(Topic, "SELECT * FROM topic ORDER BY name")
        .fetch_all(db)
        .await
}

pub async fn get_topic(db: &PgPool, topic_id: &Uuid) -> Result<Option<Topic>, sqlx::Error> {
    sqlx::query_as!(
        Topic,
        "SELECT * FROM topic WHERE id = $1",
        topic_id
    )
    .fetch_optional(db)
    .await
}

pub async fn create_message(db: &PgPool, contacts: &serde_json::Value, text: &str, topic_id: &Uuid) -> Result<Message, sqlx::Error> {
    sqlx::query_as!(
        Message,
        "INSERT INTO message (contacts, text, topic_id) VALUES ($1::jsonb, $2, $3) RETURNING *",
        contacts,
        text,
        topic_id
    )
    .fetch_one(db)
    .await
}

pub async fn get_messages(db: &PgPool, topic_id: &Uuid) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as!(
        Message, "SELECT * FROM message WHERE topic_id = $1 ORDER BY created_at DESC", topic_id
    )
    .fetch_all(db)
    .await
}
