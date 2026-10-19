// INI 的缺省值与纯解析；不访问游戏内存，不在战斗中读取文件。
#[path = "../../shared/ini.rs"]
pub mod ini;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub moveset: MovesetConfig,
    pub saed: SaedConfig,
    pub hitdata: bool,
    pub overpowered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovesetConfig {
    pub yellow_shield_aed: bool,
    pub shield_multiplier: bool,
    pub gp_recoil: bool,
    pub j_guard_aed: bool,
    pub bushi_shield_thrust_aed: bool,
    pub aed_camera_shake: bool,
    pub aed_shake_strength: u8,
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
        use ini::Value::{Bool, Integer};
        ini::rewrite(
            include_str!("../../conf/cb_moveset_to_4g.ini"),
            out,
            |section, key| {
                Some(match (section, key) {
                    ("plugin", "enabled") => Bool(self.enabled),
                    ("moveset", "yellow_shield_aed") => Bool(self.moveset.yellow_shield_aed),
                    ("moveset", "shield_multiplier") => Bool(self.moveset.shield_multiplier),
                    ("moveset", "gp_recoil") => Bool(self.moveset.gp_recoil),
                    ("moveset", "j_guard_aed") => Bool(self.moveset.j_guard_aed),
                    ("moveset", "bushi_shield_thrust_aed") => {
                        Bool(self.moveset.bushi_shield_thrust_aed)
                    }
                    ("moveset", "aed_camera_shake") => Bool(self.moveset.aed_camera_shake),
                    ("moveset", "aed_shake_strength") => {
                        Integer(self.moveset.aed_shake_strength.into())
                    }
                    ("saed", "enabled") => Bool(self.saed.enabled),
                    ("saed", "effect_gear") => Integer(self.saed.effect_gear.into()),
                    ("hitdata", "enabled") => Bool(self.hitdata),
                    ("overpowered", "enabled") => Bool(self.overpowered),
                    _ => return None,
                })
            },
        )
    }

    // 默认启用常规 4G 修改，超模恢复默认关闭。
    pub const DEFAULT: Self = Self {
        enabled: true,
        moveset: MovesetConfig {
            yellow_shield_aed: true,
            shield_multiplier: true,
            gp_recoil: true,
            j_guard_aed: true,
            bushi_shield_thrust_aed: true,
            aed_camera_shake: true,
            aed_shake_strength: 20,
        },
        saed: SaedConfig {
            enabled: true,
            effect_gear: 2,
        },
        hitdata: true,
        overpowered: false,
    };

    /// 兼容 UTF-8 BOM、CRLF、空行、# / ; 注释；缺省项沿用 DEFAULT。
    /// 写错的节、键或数值返回行号，不接受一半有效的配置。
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config = Self::DEFAULT;
        let mut section = "";
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
                if !["plugin", "moveset", "saed", "hitdata", "overpowered"].contains(&section) {
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
                ("moveset", "yellow_shield_aed") => moveset.yellow_shield_aed = boolean()?,
                ("moveset", "shield_multiplier") => moveset.shield_multiplier = boolean()?,
                ("moveset", "gp_recoil") => moveset.gp_recoil = boolean()?,
                ("moveset", "j_guard_aed") => moveset.j_guard_aed = boolean()?,
                ("moveset", "bushi_shield_thrust_aed") => {
                    moveset.bushi_shield_thrust_aed = boolean()?
                }
                ("moveset", "aed_camera_shake") => moveset.aed_camera_shake = boolean()?,
                ("moveset", "aed_shake_strength") => {
                    moveset.aed_shake_strength = integer(0, 255)? as u8
                }
                ("saed", "enabled") => saed.enabled = boolean()?,
                // 原游戏只用 1..4；0 会让冰属性的资源子类型变成不存在的 28。
                ("saed", "effect_gear") => saed.effect_gear = integer(1, 4)? as u8,
                ("hitdata", "enabled") => config.hitdata = boolean()?,
                ("overpowered", "enabled") => config.overpowered = boolean()?,
                _ => return Err(error("unknown setting; check its section and spelling")),
            }
        }
        Ok(config)
    }

    /// 关闭的功能不安装入口钩子，战斗期间无需重复判断总开关。
    pub const fn needs_hitdata(&self) -> bool {
        self.enabled && (self.hitdata || self.saed.enabled)
    }

    pub const fn needs_phial(&self) -> bool {
        self.enabled && self.saed.enabled
    }
}
