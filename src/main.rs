use anyhow::Result;
use clap::Parser;
use compio::io::AsyncWriteAt;
use futures_util::FutureExt;
use std::sync::{Arc, RwLock};
use tracing::{Level, event, instrument};

pub mod config;
pub mod error;
pub mod param;
pub mod view;

use config::Config;
use param::Args;
use std::sync::OnceLock;
use view::{AppState, History};

static SECRET: OnceLock<String> = OnceLock::new();
static YEAR: OnceLock<u16> = OnceLock::new();
static TITLE: OnceLock<String> = OnceLock::new();

#[instrument]
#[compio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let args = Args::parse();
    let config_path = std::path::Path::new(args.config.as_str());
    event!(Level::INFO, "reading configuration from {:?}", config_path);
    let config = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(config.as_str())?;
    event!(Level::INFO, "set data file to {:?}", &config.store.data);
    let history_path = std::path::PathBuf::from(&config.store.data);

    let history = {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&history_path)?;
        let reader = std::io::BufReader::new(file);
        let h: view::History = serde_json::from_reader(reader).unwrap_or_default();
        h
    };

    let list = { &history }.into();
    let list = Arc::new(RwLock::new(list));
    let history = Arc::new(RwLock::new(history));

    SECRET.get_or_init(|| config.store.secret);
    TITLE.get_or_init(|| config.meta.title);
    YEAR.get_or_init(|| config.meta.year);

    let state = AppState {
        history: history.clone(),
        board: list,
        history_path: history_path.clone(),
    };
    let app = view::router(state);
    let listener =
        compio::net::TcpListener::bind(format!("{}:{}", config.listen.address, config.listen.port))
            .await?;
    event!(
        Level::INFO,
        "listen on address {} port {}",
        config.listen.address,
        config.listen.port
    );
    cyper_axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    write_back(history, &history_path).await?;
    Ok(())
}

async fn write_back(
    history: Arc<RwLock<History>>,
    history_path: &std::path::PathBuf,
) -> Result<()> {
    serde_json::to_vec(&history.read().unwrap().clone())?;

    let data = serde_json::to_vec(&history.read().unwrap().clone())?;
    let mut file = compio::fs::File::create(history_path).await?;
    file.write_at(data, 0).await.0?;
    event!(Level::INFO, "write back to {:?}", history_path);
    Ok(())
}

async fn shutdown_signal() {
    let sigint = compio::signal::unix::signal(2);
    let sigint = std::pin::pin!(sigint);
    let sigterm = compio::signal::unix::signal(15);
    let sigterm = std::pin::pin!(sigterm);
    futures_util::select! {
         _ = sigint.fuse() => {},
         _ = sigterm.fuse() => {},
    }
    tracing::event!(tracing::Level::INFO, "gracefully shutting down");
}
