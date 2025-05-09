use super::SECRET;
use crate::error::AppError;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::TimeZone;
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
        if self.score > other.score {
            Some(std::cmp::Ordering::Greater)
        } else if self.score < other.score {
            Some(std::cmp::Ordering::Less)
        } else if self.time < other.time {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Less)
        }
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct History(HashMap<String, Vec<Record>>);

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Leaderboard(HashMap<String, Record>);

impl From<&History> for Leaderboard {
    fn from(history: &History) -> Self {
        let mut leaderboard = HashMap::new();
        for (team, records) in history.0.iter() {
            if let Some(best_record) = records.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
                leaderboard.insert(team.clone(), best_record.clone());
            }
        }
        Leaderboard(leaderboard)
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub history: Arc<RwLock<History>>,
    pub board: Arc<RwLock<Leaderboard>>,
    pub history_path: std::path::PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScorePost {
    pub team: String,
    pub score: f64,
    pub time: chrono::DateTime<chrono::Utc>,
    pub secret: String,
}

impl History {
    fn as_html(&self) -> String {
        let mut table = String::new();
        let tz_offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        for (team, records) in &self.0 {
            table.push_str(&format!(r#"<div><h3>{}</h3><table class="table table-hover"><thead><tr><th>分数</th><th>时间</th></tr></thead><tbody>"#, team));
            for record in records {
                table.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td></tr>",
                    record.score,
                    tz_offset
                        .from_utc_datetime(&record.time.naive_utc())
                        .format("%Y-%m-%d %H:%M:%S")
                ));
            }
            table.push_str("</tbody></table></div>");
        }
        table
    }
}

impl Leaderboard {
    fn as_html(&self) -> String {
        let table_head = r#"<div class="container"><h1>Ghost Hunter 2024 - JUNO Probe</h1><p>刷新页面以更新实时记录</p><div/><div class="container"><h2>排名</h2><table class="table table-hover"><thead><tr><th>队伍</th><th>分数</th><th>时间</th></tr></thead><tbody>"#;
        let table_tail = "</tbody></table><div/>";
        let mut table_body = String::new();
        let mut list: Vec<_> = self.0.iter().collect();
        list.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        let tz_offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        for (team, record) in list {
            table_body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                team,
                record.score,
                tz_offset
                    .from_utc_datetime(&record.time.naive_utc())
                    .format("%Y-%m-%d %H:%M:%S")
            ));
        }
        let table = format!("{}{}{}", table_head, table_body, table_tail);
        table
    }
}

pub async fn get_leaderboard_handler(State(state): State<AppState>) -> Result<Response, AppError> {
    let board = state.board.read().await;
    let history = state.history.read().await;
    let page = format!(
        r#"<!doctype html><html lang=zh-CN><head><link rel="icon" type="image/x-icon" href="./favicon.svg"><link href="https://cdnjs.snrat.com/ajax/libs/bootswatch/5.3.3/darkly/bootstrap.min.css" rel="stylesheet"><meta charset=utf-8 /><meta name=viewport content="width=device-width,initial-scale=1.0" /><title>Ghost Hunter 2024 排行榜</title></head><body>{}{}</body></html>"#,
        board.as_html(),
        history.as_html()
    );
    Ok(Html(page).into_response())
}

#[instrument(skip(state))]
pub async fn post_score_handler(
    State(state): State<AppState>,
    Json(score): Json<ScorePost>,
) -> Result<Response, AppError> {
    if &score.secret != SECRET.get().unwrap() {
        return Err(AppError::Unauthorized("invalid secret".to_string()));
    }
    let history = state.history.clone();
    let mut history = history.write().await;
    let records = history.0.entry(score.team.clone()).or_insert_with(Vec::new);
    records.push(Record {
        score: score.score,
        time: score.time,
    });
    event!(Level::INFO, "team {} post a new record", score.team);
    if let Some(r) = state.board.read().await.0.get(&score.team) {
        if r.score > score.score {
            return Err(AppError::Conflict(
                "score is lower than current".to_string(),
            ));
        }
    }
    let board = state.board.clone();
    let mut board = board.write().await;
    board.0.insert(
        score.team.clone(),
        Record {
            score: score.score,
            time: score.time,
        },
    );
    event!(Level::INFO, "team {} update", score.team);
    tokio::task::spawn(async move {
        let s = state.clone();
        match super::write_back(s.history.clone(), &s.history_path).await {
            Ok(_) => {}
            Err(e) => {
                event!(
                    Level::ERROR,
                    "fail to write back to {:?}: {}",
                    s.history_path,
                    e
                );
            }
        }
    });
    Ok(StatusCode::CREATED.into_response())
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", post(post_score_handler).get(get_leaderboard_handler))
        .with_state(state)
}
