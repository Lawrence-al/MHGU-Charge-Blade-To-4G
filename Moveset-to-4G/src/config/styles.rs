//! 按装备风格编号配置；4G 在配置中使用 6，引擎资源仍复用公会。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleSet(pub u8);

impl StyleSet {
    pub const COUNT: u8 = 8;
    pub const ALL: Self = Self(u8::MAX);
    pub const NONE: Self = Self(0);
    pub const FOUR_G: Self = Self(1 << 6);

    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }

    pub const fn contains(self, id: u16) -> bool {
        id < Self::COUNT as u16 && self.0 & (1 << id) != 0
    }

    pub fn set(&mut self, id: u8, enabled: bool) {
        assert!(id < Self::COUNT);
        let bit = 1 << id;
        if enabled {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }

    /// 仅预留八个编号；all,!3 / 0,2,4,6 / 0-7。
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        let mut result = Self::NONE;
        for item in text.split(',').map(str::trim) {
            match item {
                "all" => {
                    result = Self::ALL;
                    continue;
                }
                "none" => {
                    result = Self::NONE;
                    continue;
                }
                "" => return Err("use all, none, or comma-separated style IDs 0..7"),
                _ => {}
            }
            let (include, item) = match item.strip_prefix('!') {
                Some(id) => (false, id),
                None => (true, item),
            };
            let number = |s: &str| {
                s.trim()
                    .parse::<u8>()
                    .ok()
                    .filter(|id| *id < Self::COUNT)
                    .ok_or("style ID must be 0..7")
            };
            let (first, last) = match item.split_once('-') {
                Some((first, last)) => (number(first)?, number(last)?),
                None => {
                    let id = number(item)?;
                    (id, id)
                }
            };
            if first > last {
                return Err("style range is reversed");
            }
            for id in first..=last {
                result.set(id, include);
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StylesConfig {
    pub yellow_shield_aed: StyleSet,
    pub gp_recoil: StyleSet,
    pub guard_performance: StyleSet,
    pub shield_thrust_yellow: StyleSet,
    pub guard_recoil_yellow: StyleSet,
    pub aed_followups: StyleSet,
}

impl StylesConfig {
    pub const DEFAULT: Self = Self {
        yellow_shield_aed: StyleSet((1 << 3) | (1 << 6)), // 武士道和4G
        gp_recoil: StyleSet::FOUR_G,
        guard_performance: StyleSet::FOUR_G,
        shield_thrust_yellow: StyleSet::NONE,
        guard_recoil_yellow: StyleSet::NONE,
        aed_followups: StyleSet::NONE,
    };
}
