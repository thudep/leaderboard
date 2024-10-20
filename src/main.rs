use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{event, instrument, Level};

pub mod config;
pub mod error;
pub mod param;
pub mod view;

use config::Config;
use param::Args;
use view::{AppState, RecordList};

#[instrument]
#[tokio::main]
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
    event!(Level::WARN, "reading configuration from {:?}", config_path);
    let config = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(config.as_str())?;
    event!(Level::INFO, "set data file to {:?}", &config.store.data);
    if let Some(parent_dir) = std::path::Path::new(&config.store.data).parent() {
        std::fs::create_dir_all(parent_dir)?;
    }
    use tokio::sync::RwLock;
    let list = RwLock::new(
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&config.store.data)?;
            let reader = std::io::BufReader::new(file);
            let l: view::RecordList = serde_json::from_reader(reader).unwrap_or_default();
            l
        }
        .list,
    );
    let list = Arc::new(list);
    let state = AppState {
        board: list.clone(),
    };
    let app = view::router(state);
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen.address, config.listen.port))
            .await?;
    event!(
        Level::INFO,
        "listen on http://{}:{}",
        config.listen.address,
        config.listen.port
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    let file = std::fs::File::create(&config.store.data)?;
    let writer = std::io::BufWriter::new(file);
    let record = RecordList {
        list: list.read().await.clone(),
    };
    serde_json::to_writer(writer, &record)?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::event!(tracing::Level::INFO, "gracefully shutting down");
}
