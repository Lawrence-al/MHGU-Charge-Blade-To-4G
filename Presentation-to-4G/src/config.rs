//! 表现力参数的纯配置；动作类别、瓶型、请求号和挂点继续使用原任务表。
#[path = "../../shared/ini.rs"]
pub mod ini;
use crate::tasks::{ACTION_TASKS, ActionTask, BURST_ACTION_TASKS, TaskAction};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TaskSettings {
    pub enabled: bool,
    pub frame: f32,
    pub duration_frames: f32,
}

const fn task_defaults<const N: usize>(tasks: &[ActionTask]) -> [TaskSettings; N] {
    let mut values = [TaskSettings {
        enabled: true,
        frame: 0.0,
        duration_frames: 0.0,
    }; N];
    let mut i = 0;
    while i < N {
        values[i].frame = tasks[i].trigger_frame;
        if let TaskAction::CreateShieldEffect {
            duration_frames, ..
        } = &tasks[i].action
        {
            values[i].duration_frames = *duration_frames;
        }
        i += 1;
    }
    values
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FourGEffects {
    pub aed_explosion: bool,
    pub saed_explosion: bool,
    pub phial_attachment: bool,
    pub ed_blast: bool,
    pub aed_blast: bool,
    pub saed_blast: bool,
}

impl FourGEffects {
    pub const DEFAULT: Self = Self {
        aed_explosion: true,
        saed_explosion: true,
        phial_attachment: true,
        ed_blast: true,
        aed_blast: true,
        saed_blast: false,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub sounds: bool,
    pub patch_srq: bool,
    pub patch_pts: bool,
    pub volume_4a: f32,
    pub effects: bool,
    pub shield_thrust: bool,
    pub shield_thrust_duration: f32,
    pub action: [TaskSettings; ACTION_TASKS.len()],
    pub burst: [TaskSettings; BURST_ACTION_TASKS.len()],
    // 保留既有任务数组和盾突持续时间的布局。
    pub four_g: FourGEffects,
}

#[derive(Debug, PartialEq)]
pub struct ConfigError {
    pub line: usize,
    pub message: &'static str,
}

impl Config {
    pub const DEFAULT: Self = Self {
        enabled: true,
        sounds: true,
        patch_srq: true,
        patch_pts: true,
        volume_4a: -1.9,
        effects: true,
        shield_thrust: true,
        shield_thrust_duration: 30.0,
        action: task_defaults(ACTION_TASKS),
        burst: task_defaults(BURST_ACTION_TASKS),
        four_g: FourGEffects::DEFAULT,
    };

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
                if !["plugin", "sounds", "effects", "4g_effects", "impacts"].contains(&section)
                    && task_index(section, "action.", ACTION_TASKS.len()).is_none()
                    && task_index(section, "burst.", BURST_ACTION_TASKS.len()).is_none()
                {
                    return Err(error("unknown section or task number"));
                }
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| error("expected key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let boolean = || {
                value
                    .parse::<bool>()
                    .map_err(|_| error("use true or false"))
            };
            let number = |min: f32, max: f32| {
                value
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite() && (min..=max).contains(v))
                    .ok_or_else(|| error("number outside the documented range"))
            };
            match (section, key) {
                ("plugin", "enabled") => config.enabled = boolean()?,
                ("sounds", "enabled") => config.sounds = boolean()?,
                ("sounds", "patch_srq") => config.patch_srq = boolean()?,
                ("sounds", "patch_pts") => config.patch_pts = boolean()?,
                ("sounds", "volume_4a") => config.volume_4a = number(-96.0, 12.0)?,
                ("effects", "enabled") => config.effects = boolean()?,
                // 旧完整序列替换已撤去；读取旧键不再改变GU闪电，保存时移除。
                ("effects", "use_4g_sequences") => { let _ = boolean()?; }
                ("effects", "aed_4g_duration" | "saed_4g_duration") => { let _ = number(0.0, 10000.0)?; }
                ("4g_effects", "aed_explosion") => config.four_g.aed_explosion = boolean()?,
                ("4g_effects", "saed_explosion") => config.four_g.saed_explosion = boolean()?,
                ("4g_effects", "phial_attachment") => config.four_g.phial_attachment = boolean()?,
                ("4g_effects", "ed_blast") => config.four_g.ed_blast = boolean()?,
                ("4g_effects", "aed_blast") => config.four_g.aed_blast = boolean()?,
                ("4g_effects", "saed_blast") => config.four_g.saed_blast = boolean()?,
                ("effects", "shield_thrust") => config.shield_thrust = boolean()?,
                ("effects", "shield_thrust_duration") => {
                    config.shield_thrust_duration = number(0.1, 10000.0)?
                }
                // 兼容旧INI，仅忽略已删除的两项；保存时不再写回，不再安装任何旧钩子。
                ("impacts", "aed_hit_4g" | "saed_ground_4g") => { let _ = boolean()?; }
                _ => {
                    let (setting, task) = if let Some(i) =
                        task_index(section, "action.", ACTION_TASKS.len())
                    {
                        (&mut config.action[i], &ACTION_TASKS[i])
                    } else if let Some(i) = task_index(section, "burst.", BURST_ACTION_TASKS.len())
                    {
                        (&mut config.burst[i], &BURST_ACTION_TASKS[i])
                    } else {
                        return Err(error("unknown setting"));
                    };
                    match key {
                        "enabled" => setting.enabled = boolean()?,
                        "frame" => setting.frame = number(0.0, 10000.0)?,
                        "duration_frames"
                            if matches!(&task.action, TaskAction::CreateShieldEffect { .. }) =>
                        {
                            setting.duration_frames = number(0.1, 10000.0)?
                        }
                        _ => return Err(error("unknown task setting")),
                    }
                }
            }
        }
        Ok(config)
    }

    pub fn write_ini(&self, out: &mut dyn core::fmt::Write) -> core::fmt::Result {
        use ini::Value::{Bool, Float};
        ini::rewrite(
            include_str!("../../conf/cb_presentation_to_4g.ini"),
            out,
            |section, key| {
                Some(match (section, key) {
                    ("plugin", "enabled") => Bool(self.enabled),
                    ("sounds", "enabled") => Bool(self.sounds),
                    ("sounds", "patch_srq") => Bool(self.patch_srq),
                    ("sounds", "patch_pts") => Bool(self.patch_pts),
                    ("sounds", "volume_4a") => Float(self.volume_4a),
                    ("effects", "enabled") => Bool(self.effects),
                    ("4g_effects", "aed_explosion") => Bool(self.four_g.aed_explosion),
                    ("4g_effects", "saed_explosion") => Bool(self.four_g.saed_explosion),
                    ("4g_effects", "phial_attachment") => Bool(self.four_g.phial_attachment),
                    ("4g_effects", "ed_blast") => Bool(self.four_g.ed_blast),
                    ("4g_effects", "aed_blast") => Bool(self.four_g.aed_blast),
                    ("4g_effects", "saed_blast") => Bool(self.four_g.saed_blast),
                    ("effects", "shield_thrust") => Bool(self.shield_thrust),
                    ("effects", "shield_thrust_duration") => Float(self.shield_thrust_duration),
                    _ => {
                        let task = if let Some(i) =
                            task_index(section, "action.", self.action.len())
                        {
                            &self.action[i]
                        } else if let Some(i) = task_index(section, "burst.", self.burst.len()) {
                            &self.burst[i]
                        } else {
                            return None;
                        };
                        match key {
                            "enabled" => Bool(task.enabled),
                            "frame" => Float(task.frame),
                            "duration_frames" => Float(task.duration_frames),
                            _ => return None,
                        }
                    }
                })
            },
        )
    }
}

fn task_index(section: &str, prefix: &str, count: usize) -> Option<usize> {
    section
        .strip_prefix(prefix)?
        .parse::<usize>()
        .ok()
        .filter(|i| *i < count)
}
