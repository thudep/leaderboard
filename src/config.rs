//! 配置模块

use serde::Deserialize;

/// 监听配置
#[derive(Deserialize)]
pub struct Listen {
    /// 监听地址
    pub address: String,
    /// 监听端口
    pub port: u16,
}

#[derive(Deserialize)]
pub struct Static {
    pub data: String,
    pub secret: String,
}

#[derive(Deserialize)]
pub struct Metadata {
    pub year: u16,
}

/// 后端配置
#[derive(Deserialize)]
pub struct Config {
    /// 监听配置
    pub listen: Listen,
    pub store: Static,
    pub meta: Metadata,
}
