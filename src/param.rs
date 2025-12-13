//! 命令行参数配置

use clap::Parser;

/// 命令行参数
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// config file path
    #[arg(short, long, default_value = "/etc/leaderboard/leaderboard.toml")]
    pub config: String,
}
