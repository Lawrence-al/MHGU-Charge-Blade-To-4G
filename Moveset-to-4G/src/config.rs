// INI 的缺省值与纯解析；不访问游戏内存，不在战斗中读取文件。
#[path = "../../shared/ini.rs"]
pub mod ini;
#[path = "config/styles.rs"]
pub mod styles;
pub use styles::{StyleSet, StylesConfig};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub moveset: MovesetConfig,
    pub saed: SaedConfig,
    pub hitdata: bool,
    pub style_selected: bool,
    pub styles: StylesConfig,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovesetConfig {
    pub shield_multiplier: bool,
    pub j_guard_aed: bool,
    pub aed_camera_shake: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SaedConfig {
    // 第三 hit、瓶伤、逐瓶计时／贴地及特效选档共用一个开关。
    pub enabled: bool,
    pub effect_gear: u8,
}

#[derive(Debug, PartialEq)]
pub struct ConfigError {
    pub line: usize,
    pub message: &'static str,
}

impl Config {
    pub fn write_ini(&self, out: &mut dyn core::fmt::Write) -> core::fmt::Result {
        use ini::Value::{Bool, IdSet, Integer};
        ini::rewrite(
            include_str!("../../conf/cb_moveset_to_4g.ini"),
            out,
            |section, key| {
                Some(match (section, key) {
                    ("plugin", "enabled") => Bool(self.enabled),
                    ("values", "shield_multiplier") => Bool(self.moveset.shield_multiplier),
                    ("moveset", "j_guard_aed") => Bool(self.moveset.j_guard_aed),
                    ("values", "aed_camera_shake") => Bool(self.moveset.aed_camera_shake),
                    ("saed", "enabled") => Bool(self.saed.enabled),
                    ("saed", "effect_gear") => Integer(self.saed.effect_gear.into()),
                    ("hitdata", "enabled") => Bool(self.hitdata),
                    ("style", "selected") => Bool(self.style_selected),
                    ("styles", "yellow_shield_aed") => IdSet(self.styles.yellow_shield_aed.0),
                    ("styles", "gp_recoil") => IdSet(self.styles.gp_recoil.0),
                    ("styles", "guard_performance") => IdSet(self.styles.guard_performance.0),
                    ("styles", "shield_thrust_yellow") => IdSet(self.styles.shield_thrust_yellow.0),
                    ("styles", "guard_recoil_yellow") => IdSet(self.styles.guard_recoil_yellow.0),
                    ("styles", "aed_followups") => IdSet(self.styles.aed_followups.0),
                    _ => return None,
                })
            },
        )
    }

    // 黄盾变AED默认3/6，GP后退与防性默认6；超解/数值仍全局，新增三项默认关闭。
    pub const DEFAULT: Self = Self {
        enabled: true,
        moveset: MovesetConfig {
            shield_multiplier: true,
            j_guard_aed: true,
            aed_camera_shake: true,
        },
        saed: SaedConfig {
            enabled: true,
            effect_gear: 2,
        },
        hitdata: true,
        style_selected: false,
        styles: StylesConfig::DEFAULT,
    };

    /// 兼容 UTF-8 BOM、CRLF、空行、# / ; 注释；缺省项沿用 DEFAULT。
    /// 写错的节、键或数值返回行号，不接受一半有效的配置。
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config = Self::DEFAULT;
        let mut section = "";
        // 兼容旧版总开关，但不保留两层控制：明确的风格列表始终优先。
        // 仅有旧false而没有风格列表时迁移为none；旧true沿用各项新默认值。
        let mut legacy_switches = [None; 3];
        let mut explicit_styles = 0u8;
        for (index, line) in text.trim_start_matches('\u{FEFF}').lines().enumerate() {
            let line = line.split([';', '#']).next().unwrap().trim();
            let error = |message| ConfigError {
                line: index + 1,
                message,
            };
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                section = line
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .ok_or_else(|| error("expected [section]"))?;
                if ![
                    "plugin",
                    "moveset",
                    "saed",
                    "hitdata",
                    "aed_followups",
                    "overpowered",
                    "style",
                    "styles",
                    "values",
                ]
                .contains(&section)
                {
                    return Err(error("unknown section"));
                }
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| error("expected key = value"))?;
            let key = key.trim();
            let value = value.trim();
            let boolean = || {
                value
                    .parse::<bool>()
                    .map_err(|_| error("use true or false"))
            };
            let integer = |min, max| {
                value
                    .parse::<u16>()
                    .ok()
                    .filter(|v| (min..=max).contains(v))
                    .ok_or_else(|| error("integer outside the documented range"))
            };
            let moveset = &mut config.moveset;
            let saed = &mut config.saed;
            match (section, key) {
                ("plugin", "enabled") => config.enabled = boolean()?,
                ("moveset", "yellow_shield_aed" | "gp_recoil" | "guard_performance") => {
                    let index = match key {
                        "yellow_shield_aed" => 0,
                        "gp_recoil" => 1,
                        _ => 2,
                    };
                    legacy_switches[index] = Some(boolean()?);
                }
                ("values" | "moveset", "shield_multiplier") => {
                    moveset.shield_multiplier = boolean()?
                }
                ("moveset", "j_guard_aed") => moveset.j_guard_aed = boolean()?,
                ("moveset", "bushi_shield_thrust_aed") => {
                    // 旧全局启停已取消；缺少新列表时使用none，不自动开启八个风格。
                    boolean()?;
                }
                ("values" | "moveset", "aed_camera_shake") => moveset.aed_camera_shake = boolean()?,
                ("moveset", "aed_shake_strength") => {
                    // 旧配置只迁移读取；新版本固定 20，菜单和保存不再输出该键。
                    integer(0, 255)?;
                }
                ("saed", "enabled") => saed.enabled = boolean()?,
                // 原游戏只用 1..4；0 会让冰属性的资源子类型变成不存在的 28。
                ("saed", "effect_gear") => saed.effect_gear = integer(1, 4)? as u8,
                ("hitdata", "enabled") => config.hitdata = boolean()?,
                ("aed_followups" | "overpowered", "enabled") => {
                    // 旧全局键仅验证格式，改为显式选择[styles] aed_followups。
                    boolean()?;
                }
                ("style", "selected") => config.style_selected = boolean()?,
                ("styles", "yellow_shield_aed") => {
                    config.styles.yellow_shield_aed = StyleSet::parse(value).map_err(error)?;
                    explicit_styles |= 1;
                }
                ("styles", "gp_recoil") => {
                    config.styles.gp_recoil = StyleSet::parse(value).map_err(error)?;
                    explicit_styles |= 2;
                }
                ("styles", "guard_performance") => {
                    config.styles.guard_performance = StyleSet::parse(value).map_err(error)?;
                    explicit_styles |= 4;
                }
                ("styles", "shield_thrust_yellow") => {
                    config.styles.shield_thrust_yellow = StyleSet::parse(value).map_err(error)?;
                }
                ("styles", "guard_recoil_yellow") => {
                    config.styles.guard_recoil_yellow = StyleSet::parse(value).map_err(error)?;
                }
                ("styles", "aed_followups") => {
                    config.styles.aed_followups = StyleSet::parse(value).map_err(error)?;
                }
                _ => return Err(error("unknown setting; check its section and spelling")),
            }
        }
        for (index, set) in [
            &mut config.styles.yellow_shield_aed,
            &mut config.styles.gp_recoil,
            &mut config.styles.guard_performance,
        ]
        .into_iter()
        .enumerate()
        {
            if explicit_styles & (1 << index) == 0 && legacy_switches[index] == Some(false) {
                *set = StyleSet::NONE;
            }
        }
        Ok(config)
    }

    /// HDP是所有盾斧共用的资源；始终注册0x50，战斗功能可以独立关闭。
    pub const fn needs_hitdata(&self) -> bool {
        true
    }

    pub fn needs_phial(&self) -> bool {
        self.enabled && self.saed.enabled
    }
}
