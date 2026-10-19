// 手动编辑与菜单保存均使用 MHGU 存档根目录；战斗期间的配置在启动时固定。
use crate::{config::Config, config_io};
use forge::os::sync::OnceLock;

// 完整功能注释超过 4 KiB，留足 UTF-8 中文及用户补充注释的空间。
static CONFIG: OnceLock<Config> = OnceLock::new();

/// 必须在安装任何补丁／钩子之前调用，之后仅读取不可变配置。
pub fn initialize() -> &'static Config {
    CONFIG.get_or_init(|| load())
}

pub fn get() -> &'static Config {
    CONFIG.get().unwrap()
}

fn load() -> Config {
    config_io::MOVESET.load(Config::DEFAULT, Config::parse)
}

pub fn save(config: &Config) -> Result<(), &'static str> {
    let mut text = crate::config::ini::Text::<8193>::new();
    config
        .write_ini(&mut text)
        .map_err(|_| "Configuration is too long.")?;
    Config::parse(text.as_str()).map_err(|e| e.message)?;
    config_io::MOVESET.save_text(text.as_str())
}
