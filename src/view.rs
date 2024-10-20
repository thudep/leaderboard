use crate::error::AppError;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
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

type Leaderboard = HashMap<String, Record>;

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

pub async fn get_leaderboard_handler(State(state): State<AppState>) -> Result<Response, AppError> {
    let board = state.board.read().await;
    let mut list: Vec<_> = board.iter().collect();
    list.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    let list: Vec<_> = list.iter().map(|(k, v)| (k, v)).collect();
    Ok(Json(list).into_response())
}

#[instrument(skip(state))]
pub async fn post_score_handler(
    State(state): State<AppState>,
    Json(score): Json<ScorePost>,
) -> Result<Response, AppError> {
    if let Some(r) = state.board.read().await.get(&score.team) {
        if r.score > score.score {
            return Err(AppError::Conflict(
                "score is lower than current".to_string(),
            ));
        }
    }
    let mut board = state.board.write().await;
    board.insert(
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
