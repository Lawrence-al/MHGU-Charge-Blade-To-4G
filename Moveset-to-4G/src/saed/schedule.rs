// 瓶爆的累计时间及局部位置；数值来自 4G 启动初始化后的表。
// 此模块不读游戏内存，便于独立验证 1～10 瓶的时序。

/// 代码内固定的时间／位置；从首爆当帧人物位置算起，不通过 INI 调整。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub impact_first_delay: f32,
    pub impact_interval: f32,
    pub impact_first_forward: f32,
    pub impact_step: f32,
    pub element_first_delay: f32,
    pub element_first_forward: f32,
    pub element_forward_scale: f32,
    pub element_side_scale: f32,
    pub end_delay: f32,
}

impl Settings {
    pub const DEFAULT: Self = Self {
        impact_first_delay: 0.0,
        impact_interval: 9.0,        // 旧 SH 总长 21 = 6 + 6 + 9。
        impact_first_forward: 270.0, // 4G 进程先前移 200，再加首项 70。
        impact_step: 100.0,
        element_first_delay: 46.0,
        element_first_forward: 300.0, // 200 + 100。
        element_forward_scale: 1.0,
        element_side_scale: 1.0,
        end_delay: 9.0,
    };
}

pub use cb_game::player::PhialKind;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    pub at: f32,
    pub side: f32,
    pub forward: f32,
}

// 4G 属性瓶原表：1、2、3 个一组；这里保存相对第一爆的位移。
// 第四组由前三组外推：纵向继续增加 120，半宽 160，四点均匀分布。
// 原第三组先两边后中间，第四组也先两边后内部。
const ELEMENT_BLASTS: [Blast; 10] = [
    Blast {
        at: 0.0,
        side: 0.0,
        forward: 0.0,
    },
    Blast {
        at: 18.0,
        side: 80.0,
        forward: 120.0,
    },
    Blast {
        at: 20.0,
        side: -80.0,
        forward: 120.0,
    },
    Blast {
        at: 38.0,
        side: -120.0,
        forward: 240.0,
    },
    Blast {
        at: 40.0,
        side: 120.0,
        forward: 240.0,
    },
    Blast {
        at: 42.0,
        side: 0.0,
        forward: 240.0,
    },
    Blast {
        at: 58.0,
        side: -160.0,
        forward: 360.0,
    },
    Blast {
        at: 60.0,
        side: 160.0,
        forward: 360.0,
    },
    Blast {
        at: 62.0,
        side: -160.0 / 3.0,
        forward: 360.0,
    },
    Blast {
        at: 64.0,
        side: 160.0 / 3.0,
        forward: 360.0,
    },
];

pub fn blast_with_settings(kind: PhialKind, index: u8, settings: &Settings) -> Blast {
    match kind {
        // 4G 初始化前移 200，再加原表 70、170、270……。
        PhialKind::Impact => Blast {
            at: index as f32 * settings.impact_interval,
            side: 0.0,
            forward: settings.impact_first_forward + index as f32 * settings.impact_step,
        },
        PhialKind::Element => {
            let mut blast = ELEMENT_BLASTS[index as usize];
            blast.forward =
                settings.element_first_forward + blast.forward * settings.element_forward_scale;
            blast.side *= settings.element_side_scale;
            blast
        }
    }
}

fn first_delay(kind: PhialKind, settings: &Settings) -> f32 {
    match kind {
        PhialKind::Impact => settings.impact_first_delay,
        PhialKind::Element => settings.element_first_delay,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Wait,
    Blast { index: u8, remaining: u8 },
    Stop,
}

#[derive(Clone, Copy)]
pub struct Sequence {
    pub kind: PhialKind,
    pub settings: Settings,
    total: u8,
    next: u8,
    elapsed: f32,
    expires: f32,
}

impl Sequence {
    pub fn with_settings(kind: PhialKind, total: u8, settings: Settings) -> Self {
        Self {
            kind,
            settings,
            total,
            next: 0,
            elapsed: -first_delay(kind, &settings),
            expires: 0.0,
        }
    }

    /// 与 4G 一样每次更新至多爆一瓶；用累计时间避免逐次取整造成漂移。
    pub fn advance(&mut self, delta: f32) -> Step {
        if delta == 0.0 {
            return Step::Wait;
        }
        self.elapsed += delta;
        if self.next < self.total {
            if self.elapsed < blast_with_settings(self.kind, self.next, &self.settings).at {
                return Step::Wait;
            }
            let index = self.next;
            self.next += 1;
            if index == 0 {
                // 首爆在第一个有效更新触发后归零；后续间隔相对这一次实际首爆。
                self.elapsed = 0.0;
            }
            self.expires = self.elapsed + self.settings.end_delay;
            Step::Blast {
                index,
                remaining: self.total - self.next,
            }
        } else if self.elapsed >= self.expires {
            Step::Stop
        } else {
            Step::Wait
        }
    }
}

/// yaw 的正弦／余弦构成水平朝向，side 沿右方向旋转。
pub fn world_xz(origin: [f32; 3], heading: [f32; 2], local: Blast) -> [f32; 3] {
    let [sin, cos] = heading;
    [
        origin[0] + cos * local.side + sin * local.forward,
        origin[1],
        origin[2] + cos * local.forward - sin * local.side,
    ]
}

#[cfg(not(target_arch = "arm"))]
pub fn blast(kind: PhialKind, index: u8) -> Blast {
    blast_with_settings(kind, index, &Settings::DEFAULT)
}

#[cfg(not(target_arch = "arm"))]
impl Sequence {
    pub fn new(kind: PhialKind, total: u8) -> Self {
        Self::with_settings(kind, total, Settings::DEFAULT)
    }
}
