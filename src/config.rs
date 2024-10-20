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

/// 后端配置
#[derive(Deserialize)]
pub struct Config {
    /// 监听配置
    pub listen: Listen,
    /// 上传配置
    pub data: String,
    /// 密钥
    pub secret: String,
}
