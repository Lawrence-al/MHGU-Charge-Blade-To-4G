//! 两个盾斧插件按运行环境选择存档或plugins配置读写。
#[path = "config_storage.rs"]
mod storage;
#[allow(unused_imports)]
pub use storage::{Location, MAX_BYTES, MOVESET, PRESENTATION, STYLE};
