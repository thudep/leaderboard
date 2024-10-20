use crate::error::AppError;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{event, instrument, Level};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Record {
    pub score: f64,
    pub time: chrono::DateTime<chrono::Utc>,
}

impl PartialOrd for Record {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RecordList {
    pub list: Leaderboard,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Leaderboard(HashMap<String, Record>);

#[derive(Debug, Clone)]
pub struct AppState {
    pub board: Arc<RwLock<Leaderboard>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScorePost {
    pub team: String,
    pub score: f64,
    pub time: chrono::DateTime<chrono::Utc>,
}

impl IntoResponse for Leaderboard {
    fn into_response(self) -> Response {
        let table_head =
            "<table><thead><tr><th>Team</th><th>Score</th><th>Time</th></tr></thead><tbody>";
        let table_tail = "</tbody></table>";
        let mut table_body = String::new();
        let mut list: Vec<_> = self.0.iter().collect();
        list.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (team, record) in list {
            table_body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                team, record.score, record.time
            ));
        }
        let table = format!(
            r#"<!doctype html><html lang=zh-CN><head><meta charset=utf-8 /><meta name=viewport content="width=device-width,initial-scale=1.0" /><title>Ghost Hunter 排行榜</title></head><body>{}{}{}<body/></html>"#,
            table_head, table_body, table_tail
        );
        Html(table).into_response()
    }
}

pub async fn get_leaderboard_handler(State(state): State<AppState>) -> Result<Response, AppError> {
    let board = state.board.read().await;
    Ok(board.clone().into_response())
}

#[instrument(skip(state))]
pub async fn post_score_handler(
    State(state): State<AppState>,
    Json(score): Json<ScorePost>,
) -> Result<Response, AppError> {
    if let Some(r) = state.board.read().await.0.get(&score.team) {
        if r.score > score.score {
            return Err(AppError::Conflict(
                "score is lower than current".to_string(),
            ));
        }
    }
    let mut board = state.board.write().await;
    board.0.insert(
        score.team.clone(),
        Record {
            score: score.score,
            time: score.time,
        },
    );
    event!(
        Level::INFO,
        "team {} posted score {}",
        score.team,
        score.score
    );
    Ok(StatusCode::CREATED.into_response())
}
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", post(post_score_handler).get(get_leaderboard_handler))
        .with_state(state)
}
