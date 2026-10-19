use crate::{config::Config, config_io};
use forge::os::sync::OnceLock;
static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn initialize() -> &'static Config {
    CONFIG.get_or_init(|| config_io::PRESENTATION.load(Config::DEFAULT, Config::parse))
}

#[inline(never)]
pub fn get() -> &'static Config {
    CONFIG.get().unwrap()
}

pub fn save(config: &Config) -> Result<(), &'static str> {
    let mut text = crate::config::ini::Text::<8193>::new();
    config
        .write_ini(&mut text)
        .map_err(|_| "Configuration is too long.")?;
    Config::parse(text.as_str()).map_err(|e| e.message)?;
    config_io::PRESENTATION.save_text(text.as_str())
}
