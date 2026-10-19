// action.rs
// 动作相关 - 盾斧的按帧任务表与执行。目前挂两个动作：
//
//   1) sub_611DABC8（= 4G SAED_action）：大姐/超解本体        → ACTION_TASKS
//   2) sub_611DA204（= 4G sub_CAC594）：瓶爆子动作            → BURST_ACTION_TASKS
//
// 结构：actionState（0=起手那一帧 / 1..=后续各段）
//         └─ variant（哪个子动作）→ 到第几帧 → 做什么（播 SE / 创建盾牌特效）
//
// 两张表分开是因为两个动作的 actionState（`*(u8*)(player+5)`）和 variant 编号空间重叠，
// 混在一张表里会互相误命中。瓶爆子动作除 0x4C 外，还要求"有瓶"（`*(u8*)(player+6) != 0`）。

use ::core::sync::atomic::{AtomicU32, Ordering};

use crate::{config::{Config, TaskSettings}, config_file, effect, sound, state::STATE};
use cb_game::{
    player::ChargeBladePlayer,
    sound::{MssEvent, SoundSequence},
};

// ============================================================================
// 任务表
// ============================================================================

use crate::tasks::{ACTION_TASKS, ActionTask, BLOCK_RULES, BURST_ACTION_TASKS, TaskAction};

// ============================================================================
// 运行状态
// ============================================================================

/// 一张任务表的执行状态；两组动作各持有一份，继续使用原子字段。
struct ActionRunner {
    tasks: &'static [ActionTask],
    require_phial: bool,
    fired_mask: AtomicU32,
    previous_state: AtomicU32,
}

impl ActionRunner {
    const fn new(tasks: &'static [ActionTask], require_phial: bool) -> Self {
        Self {
            tasks,
            require_phial,
            fired_mask: AtomicU32::new(0),
            previous_state: AtomicU32::new(0xFFFF_FFFF),
        }
    }

    /// 通用按帧任务执行（两张表共用）
    ///
    /// 任务表、有瓶门槛、已执行标记与上一阶段均由本 ActionRunner 管理。
    ///
    /// 说明：判定用的是"当前动作帧 >= 任务帧 + 本次动作未执行过"，所以即使某一帧被漏掉
    /// （掉帧）也能补上。
    unsafe fn run(&self, player: ChargeBladePlayer, variant: u32, action_state: u32, config: &Config, settings: &[TaskSettings]) {
        // actionState 回到 0 表示新一次动作开始 → 清空"已执行"标记
        let prev = self.previous_state.swap(action_state, Ordering::Relaxed);
        if action_state == 0 && prev != 0 {
            self.fired_mask.store(0, Ordering::Relaxed);
        }

        // 记下玩家对象，play_se* 需要用它推 SE 归属对象
        STATE.set_player(player);

        if variant >= 32 {
            return;
        }

        let missing_phial = self.require_phial && !unsafe { player.has_phial() };

        let variant_bit = 1u32 << variant;
        let current_frame = unsafe { player.motion_frame() };
        let fired = self.fired_mask.load(Ordering::Relaxed);

        for (index, task) in self.tasks.iter().enumerate() {
            let setting = &settings[index];
            if !setting.enabled || match &task.action {
                TaskAction::PlaySe { .. } => !config.sounds,
                TaskAction::CreateShieldEffect { .. } => !config.effects,
            } { continue; }
            if task.action_state != action_state || task.variant_mask & variant_bit == 0 {
                continue;
            }

            // 0x4C 无瓶时也要播放；其它瓶爆任务仍要求有瓶。
            if missing_phial
                && !matches!(
                    &task.action,
                    TaskAction::PlaySe {
                        request_id: 0x4C,
                        ..
                    }
                )
            {
                continue;
            }

            let task_bit = 1u32 << index;
            if fired & task_bit != 0 {
                continue;
            }

            // trigger_frame == 0.0 表示"进入这个阶段就执行"，不做帧比较
            if setting.frame > 0.0 && current_frame < setting.frame {
                continue;
            }

            self.fired_mask.fetch_or(task_bit, Ordering::Relaxed);
            TASK_RUN_COUNT.fetch_add(1, Ordering::Relaxed);
            unsafe { execute_task(player, &task.action, setting.duration_frames) };
        }
    }
}

static PHIAL_RUNNER: ActionRunner = ActionRunner::new(ACTION_TASKS, false);
static BURST_RUNNER: ActionRunner = ActionRunner::new(BURST_ACTION_TASKS, true);
static TASK_RUN_COUNT: AtomicU32 = AtomicU32::new(0);

/// 由 sub_611DABC8 的钩子每帧调用（大姐/超解本体 → [`ACTION_TASKS`]）
///
/// 参数：`player_actor` 玩家对象
/// 参数：`variant` 子动作编号（第二参数 R1）
/// 参数：`action_state` 动作阶段（0=起手帧 / 1=后续帧）
pub unsafe fn on_phial_action_frame(player: ChargeBladePlayer, variant: u32, action_state: u32) {
    let config = config_file::get();
    unsafe { PHIAL_RUNNER.run(player, variant, action_state, config, &config.action) }
}

/// 由 sub_611DA204 的钩子每帧调用（瓶爆子动作 → [`BURST_ACTION_TASKS`]）
///
/// 参数：`player_actor` 玩家对象
/// 参数：`variant` 子动作编号（第二参数 R1）
/// 参数：`action_state` 动作阶段（0=起手 / 1=中间 / 2=爆瓶 / 3=收尾）
///
/// 和 [`on_phial_action_frame`] 一样，只是多一个"有瓶"门槛（`require_phial = true`）：
/// `*(u8*)(player+6) == 0`（没瓶）时只允许 0x4C 执行。
pub unsafe fn on_burst_action_frame(player: ChargeBladePlayer, variant: u32, action_state: u32) {
    let config = config_file::get();
    unsafe { BURST_RUNNER.run(player, variant, action_state, config, &config.burst) }
}

/// 执行一条任务（要加新动作类型就在 match 里加分支）
///
/// 参数：`player_actor` 本次动作的玩家对象，特效直接使用其盾牌
/// 参数：`task_action` 任务内容
unsafe fn execute_task(player: ChargeBladePlayer, task_action: &TaskAction, duration_frames: f32) {
    match task_action {
        TaskAction::PlaySe {
            request_id,
            bank,
            attach_index,
        } => {
            unsafe { sound::play_se_ex(*request_id, *bank, *attach_index, 0) };
        }
        TaskAction::CreateShieldEffect {
            effect_id,
            element_effect_id,
            duration_frames: _,
        } => {
            unsafe {
                effect::create_timed_on_shield(
                    player,
                    *effect_id,
                    *element_effect_id,
                    duration_frames,
                )
            };
        }
    }
}

/// 累计执行任务次数
#[cfg(feature = "debug-logs")]
pub fn get_task_run_count() -> u32 {
    TASK_RUN_COUNT.load(Ordering::Relaxed)
}

// ============================================================================
// 拦截规则表（把游戏自己排期的某一条音效掐掉）
// ============================================================================

/// 判断这条 mss 事件是否命中拦截规则
///
/// 参数：`sound_seq_ctrl` 音序控制器（mss）
/// 参数：`command_bytes` mss 事件起始地址（SeCommand_Dispatch 的第 3 个参数）
/// 返回：命中的规则下标（None = 不拦）
///
/// 事件布局（按反编译逐字节核对）：
/// `u8 opcode | u8 flag | u8 scanCount | u8 pad | u32 scan[scanCount] | 参数区`，
/// 参数区第 1 个 u16 = bank，第 2 个 u16 = request_id。
pub unsafe fn find_block_rule(
    sound_seq_ctrl: SoundSequence,
    command_bytes: *const u8,
) -> Option<usize> {
    if command_bytes.is_null() {
        return None;
    }

    let event = MssEvent::from_address(command_bytes as u32);
    let opcode = unsafe { event.opcode() };
    let bank = unsafe { event.bank() };
    let request_id = unsafe { event.request_id() };
    let current_frame = unsafe { sound_seq_ctrl.current_frame() };
    let prev_frame = unsafe { sound_seq_ctrl.previous_frame() };

    BLOCK_RULES.iter().position(|rule| {
        rule.opcode == opcode
            && rule.bank == bank
            && rule.request_id == request_id
            && prev_frame < rule.frame
            && rule.frame <= current_frame
    })
}
