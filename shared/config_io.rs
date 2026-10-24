//! 两个盾斧插件共用同一套存档根目录读写；不读取 SD 或 ROMFS 配置。
#[path = "config_storage.rs"]
mod storage;
#[allow(unused_imports)]
pub use storage::{Location, MAX_BYTES, MOVESET, PRESENTATION, STYLE};
