//! 命令行参数配置

use clap::Parser;

/// 命令行参数
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "/usr/local/etc/veloquent/veloquent.toml")]
    pub config: String,
}
