// 2026-10-11 重构前执行器的最小回归基准；只在宿主测试中编译。
// 保留这两段旧实现用于独立行为对照，不进入插件 NRO。
pub mod constant {
// constant.rs
// 常量定义模块 - 集中管理所有地址、字段偏移与枚举

// ============================================================================
// 游戏函数地址（IDA 地址 − 0x60000000，运行时 = runtime::game_addr(偏移)）
// ============================================================================

/// SeCommand_Dispatch 的偏移（IDA 604D74F8）
///
/// 签名：`(sequence_slot: u32, sound_seq_ctrl: u32, command_bytes: *const u8) -> u32`
/// 每个 mss 事件都会进这里，是"取 SE 归属对象"和"拦事件"的位置。
pub const SE_COMMAND_DISPATCH: u32 = 0x004D74F8;

/// SeState_Update 的偏移（IDA 604D5E90）
///
/// 签名：`(sound_seq_ctrl: u32) -> u32`
/// 每帧推进动作帧并派发到帧的事件。
pub const SE_STATE_UPDATE: u32 = 0x004D5E90;

/// Action_and_efx_init 的偏移（IDA 60281998）
///
/// 签名：`(player_actor: u32, action_key: u32) -> u32`
/// 动作起手时调用，`action_key` 就是 0x1412/0x1405 这类 key。
pub const ACTION_EFX_INIT: u32 = 0x00281998;

/// WriteSfxId2Pool 的偏移（IDA 604F0BE8）
///
/// 签名：`(se_param_mgr, se_owner, request_id, attach_index, param_a5, pool_type, bank) -> u32`
pub const WRITE_SFX_ID_TO_POOL: u32 = 0x004F0BE8;

/// rSoundRequest::二进制资源绑定（sub_60B67AE4）；只在 SRQR 读取及验证后调用。
pub const SRQ_RESOURCE_BIND: u32 = 0x00B67AE4;

/// rFestaPelTiedSe::load（sub_604D923C）；返回 1 表示 XFS 已反序列化成功。
pub const PTS_RESOURCE_LOAD: u32 = 0x004D923C;

/// MtResource::mPath（内嵌 64 字节字符数组；sub_608875F0 返回 object+8）。
pub const RESOURCE_PATH: u32 = 0x0008;

/// rSoundRequest::mData：绑定前为完整 SRQR 文件缓冲区。
pub const SRQ_RESOURCE_DATA: u32 = 0x0064;

/// rFestaPelTiedSe::mData（MtArray）：对象数及 cFestaPelTiedSe 指针数组。
pub const PTS_RESOURCE_COUNT: u32 = 0x0068;
pub const PTS_RESOURCE_ENTRIES: u32 = 0x0074;

/// dword_61896F5C 的偏移：SE 参数管理器（播放池在它 +0xEC，64 槽 × 0x24 字节）
pub const G_SE_PARAM_MANAGER: u32 = 0x01896F5C;

/// sub_611DABC8 的偏移（IDA 611DABC8 → 0x611DABC8 − 0x60000000 = 0x011DABC8）
///
/// 签名：`(player_actor: u32, variant: u32) -> u32`
/// 盾斧"瓶爆类"动作的每帧更新函数，对应 4G 的 `SAED_action`：
/// 内部先用 `actionState`（0=起手帧 / 1=后续帧）分两段，再按 `variant` 跳转。
pub const CB_PHIAL_ACTION: u32 = 0x011DABC8;

/// sub_611DA204 的偏移（IDA 611DA204 → 0x011DA204）
///
/// 签名：`(player_actor: u32, variant: u32)`
/// 盾斧"瓶爆子动作"的每帧更新函数，对应 4G 的 `sub_CAC594`
/// （4G 里由 CB_action 的 case 0x44/0x46/0x4B 进入）。
/// 内部按 `actionState`（`*(u8*)(player+5)`）分 0/1/2/3 段，
/// 起手帧把"有没有瓶"写进 [`PLAYER_PHIAL_FLAG`]。
pub const CB_BURST_ACTION: u32 = 0x011DA204;

/// 特效创建参数的构造 / 析构（sub_60040B10 / sub_60040B50）
pub const EFFECT_PARAM_CONSTRUCT: u32 = 0x00040B10;
pub const EFFECT_PARAM_DESTRUCT: u32 = 0x00040B50;

/// sub_604A1E08：按 PEL 组和 effectId 创建武器特效
/// 签名：`(effect_shell, pel_handle, group, effect_id, create_params, sound_table) -> node`
pub const CREATE_WEAPON_EFFECT: u32 = 0x004A1E08;

/// uMHEffectCore 每帧更新与析构（sub_60043224 / sub_6004201C）
pub const EFFECT_CORE_UPDATE: u32 = 0x00043224;
pub const EFFECT_CORE_DESTRUCT: u32 = 0x0004201C;

/// sub_60170968：sHitcheck 确认攻击判定与怪物碰撞后处理命中
/// 签名：`(hitcheck, hit_attack, monster, contact, body_contact, attack_contact, hit_part, shape_index) -> u32`
pub const ATTACK_HIT_MONSTER_DISPATCH: u32 = 0x00170968;

/// sub_6032A400：停止节点；第二参数 0 走 PSL 同样的粒子收尾路径，1 立即释放
pub const STOP_EFFECT: u32 = 0x0032A400;

/// IS_ack 的偏移（IDA 60280C1C）
///
/// 帧判定：`(player_actor: u32, frame: f32) -> bool`
/// 返回"本动作是否已经走到第 frame 帧"（内部和 mMotion[0].mFrameMax/mLoopFrame 比较，带循环回绕）。
///
/// 注意：本插件是按 **soft-float ABI** 编译的（target json 里 `llvm-floatabi: soft`），
/// 带 f32 参数直接调游戏函数会传错寄存器，所以现在**不直接调用它**，
/// 帧判定改成自己读 [`PLAYER_MOTION_FRAME`] 比较。
#[allow(dead_code)] // 留着备用：以后若改成 hard-float ABI 或做帧窗口判定会用到
pub const IS_ACK: u32 = 0x00280C1C;

// ============================================================================
// 游戏对象字段偏移
// ============================================================================

/// 玩家对象 → 音序控制器（uFestActorModel::mSndSeqCtrl，类型 cFestaSoundSequence）
pub const PLAYER_TO_SOUND_SEQ: u32 = 0x111C;

/// 音序控制器 → SE 归属对象（cFestaSoundSequence::mOwner）
pub const SOUND_SEQ_TO_OWNER: u32 = 0x0164;

/// 音序控制器 → 当前整数帧（游戏判定"事件到没到帧"用的就是它）
pub const SOUND_SEQ_CURRENT_FRAME: u32 = 0x0090;

/// 音序控制器 → 上一个整数帧（游戏用它做"跨帧触发"判断：`prev < 行帧 <= cur`）
pub const SOUND_SEQ_PREV_FRAME: u32 = 0x00B0;

/// 玩家对象 → 当前动作帧（mMotion[0].mFrame；uModel::Motion 在 +0x4B0，mFrame 在 +0x50）
pub const PLAYER_MOTION_FRAME: u32 = 0x0500;

/// mMotion[0] 的动画编号（sub_6053A058 读取的 a1[602]，包括动画组与组内编号）
pub const PLAYER_MOTION_ID: u32 = 0x04B4;

/// 玩家对象 → actionState（= `BYTE1(*(u32*)(player+4))`）
///
/// 0 = 这个动作的起手那一帧，1 = 之后每帧，其它值表示不在这个动作里。
/// （4G 的对应字段在 `*(u32*)(player+0xE30)+5`，对象布局不同，别照搬。）
pub const PLAYER_ACTION_STATE: u32 = 0x0005;

/// 玩家对象 → 当前 actionID（盾斧动作函数 switch 的那个值）
pub const PLAYER_ACTION_ID: u32 = 0x250A;

/// 玩家对象 → 有瓶标志（= `*(u8*)(player+6)`，也就是 `BYTE2(*(u32*)(player+4))`）
///
/// 动作起手帧写入，有瓶为 1、没瓶为 0
/// （`sub_611DA204` 的 case 0：`if (!(vtable+2160)(player) && (vtable+460)(player)) *(u8*)(player+6) = 1;`）。
/// 0 = 没瓶 → 瓶爆特效（4G 特效 0x37）和它的伴生音效 0x4A 都不该出。
pub const PLAYER_PHIAL_FLAG: u32 = 0x0006;

/// 玩家对象 → 当前装备参数对象；sub_6027C708 通过它查询武器参数
pub const PLAYER_TO_EQUIP_PARAMS: u32 = 0x13C4;

/// 装备参数对象 → 盾斧瓶型（s16）：0 为榴弹瓶，非 0 为强属性瓶。
/// 原生查询参数 27：对象 + 4 + 796 + 27 * 2 = 对象 + 0x356。
pub const EQUIP_PARAMS_PHIAL_TYPE: u32 = 0x0356;

/// 盾斧玩家 → 盾牌对象；红盾动作 sub_611D7F70 使用同一字段
pub const PLAYER_TO_SHIELD: u32 = 0x23A4;

/// 玩家 → 武器特效 shell（uShellPlEffectW14）
pub const PLAYER_TO_WEAPON_EFFECT_SHELL: u32 = 0x2634;

/// 武器特效 shell → 第 0 组 PEL 资源句柄（sub_604A1E50）
pub const WEAPON_EFFECT_SHELL_TO_PEL: u32 = 0x1388;

/// 对象虚表 → 取得对象引用的函数（当前 sub_6006BE88 返回 object+0xFD0）
pub const OBJECT_GET_REFERENCE_VTABLE_OFFSET: u32 = 0x0130;

/// uMHEffectCore 状态；2 表示已经停止生成，等待粒子收尾
pub const EFFECT_NODE_STATE: u32 = 0x0030;

/// uMHEffectCore 的更新帧增量；sub_60043224 将它累加到节点已播放帧数。
pub const EFFECT_NODE_DELTA_FRAME: u32 = 0x001C;

/// 玩家内嵌的两组 MhHitAtk；sub_602990BC 构造，盾斧 HDP 使用这两组攻击判定
pub const PLAYER_WEAPON_HIT_PRIMARY: u32 = 0x3020;
pub const PLAYER_WEAPON_HIT_SECONDARY: u32 = 0x30D8;

/// MhHitAtk 命中结果；sub_6016919C 每帧先重置为 0xFF，碰撞处理再写入结果
pub const HIT_ATTACK_RESULT: u32 = 0x0006;
pub const HIT_RESULT_NONE: u8 = 0xFF;

// ============================================================================
// 枚举
// ============================================================================

/// uPlayerQuest14 的 vtable（模块内偏移），用于确认对象是不是玩家
pub const VFT_U_PLAYER_QUEST14: u32 = 0x01750D20;

/// mss 事件 0x66（SE_Play）对应的池类型（WriteSfxId2Pool 第 6 个参数）
pub const POOL_TYPE_SE_PLAY: u32 = 4;

/// 动作 key（mss / psl / pts 共用同一套编号，也是 Action_and_efx_init 的第 2 个参数）
pub mod action_key {
    /// 大姐（AED）：mss 0x1412
    pub const AED: u32 = 0x1412;
    /// 超解（SAED）：mss 0x1405
    pub const SAED: u32 = 0x1405;
}
}
pub mod old_action {
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

use forge::log::prelude::error;

use crate::constant;
use crate::effect;
use crate::memory;
use crate::runtime;
use crate::sound;

// ============================================================================
// 任务表
// ============================================================================

pub use crate::tasks::{
    ACTION_TASKS, ActionTask, BLOCK_RULES, BURST_ACTION_TASKS, BlockRule, TaskAction,
};

// ============================================================================
// 运行状态
// ============================================================================

/// 本次动作里哪些任务已经执行过（按 [`ACTION_TASKS`] 下标置位）
static FIRED_MASK: AtomicU32 = AtomicU32::new(0);

/// 上一次见到的 actionState（用来识别"新一次动作开始"）
static PREV_ACTION_STATE: AtomicU32 = AtomicU32::new(0xFFFF_FFFF);

/// 瓶爆子动作（[`BURST_ACTION_TASKS`]）里哪些任务已经执行过
static BURST_FIRED_MASK: AtomicU32 = AtomicU32::new(0);

/// 瓶爆子动作上一次见到的 actionState
static BURST_PREV_ACTION_STATE: AtomicU32 = AtomicU32::new(0xFFFF_FFFF);

/// 累计执行任务次数
static TASK_RUN_COUNT: AtomicU32 = AtomicU32::new(0);

// ============================================================================
// 对外操作
// ============================================================================

/// 通用按帧任务执行（两张表共用）
///
/// 参数：`tag` 日志前缀（`phial` / `burst`，用来区分是哪张表）
/// 参数：`tasks` 任务表
/// 参数：`fired_mask` 这张表的"已执行"标记
/// 参数：`prev_state` 这张表的"上一次 actionState"
/// 参数：`require_phial` 是否要求除 0x4C 外的任务"有瓶"才执行
///
/// 说明：判定用的是"当前动作帧 >= 任务帧 + 本次动作未执行过"，所以即使某一帧被漏掉
/// （掉帧）也能补上；日志里会打出触发时的真实帧号，方便核对。
unsafe fn run_tasks(
    _tag: &str,
    tasks: &[ActionTask],
    player_actor: u32,
    variant: u32,
    action_state: u32,
    fired_mask: &AtomicU32,
    prev_state: &AtomicU32,
    require_phial: bool,
) {
    // actionState 回到 0 表示新一次动作开始 → 清空"已执行"标记
    let prev = prev_state.swap(action_state, Ordering::Relaxed);
    if action_state == 0 && prev != 0 {
        fired_mask.store(0, Ordering::Relaxed);
        // error!("se-src-plugin: {_tag} action start (variant {variant})");
    }

    // 记下玩家对象，play_se* 需要用它推 SE 归属对象
    runtime::set_master_player(player_actor);

    if variant >= 32 {
        return;
    }

    let missing_phial = require_phial
        && unsafe { memory::read_u8(player_actor + constant::PLAYER_PHIAL_FLAG) } == 0;

    let variant_bit = 1u32 << variant;
    let current_frame = unsafe { memory::read_f32(player_actor + constant::PLAYER_MOTION_FRAME) };
    let fired = fired_mask.load(Ordering::Relaxed);

    for (index, task) in tasks.iter().enumerate() {
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
        if task.trigger_frame > 0.0 && current_frame < task.trigger_frame {
            continue;
        }

        fired_mask.fetch_or(task_bit, Ordering::Relaxed);
        let total = TASK_RUN_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        /*
        error!(
            "se-src-plugin: {_tag} task#{index} actionState {action_state} variant {variant} frame {current_frame:.2} (target {:.2})",
            task.trigger_frame
        );
        */
        unsafe { execute_task(player_actor, &task.action, total) };
    }
}

/// 由 sub_611DABC8 的钩子每帧调用（大姐/超解本体 → [`ACTION_TASKS`]）
///
/// 参数：`player_actor` 玩家对象
/// 参数：`variant` 子动作编号（第二参数 R1）
/// 参数：`action_state` 动作阶段（0=起手帧 / 1=后续帧）
pub unsafe fn on_phial_action_frame(player_actor: u32, variant: u32, action_state: u32) {
    unsafe {
        run_tasks(
            "phial",
            ACTION_TASKS,
            player_actor,
            variant,
            action_state,
            &FIRED_MASK,
            &PREV_ACTION_STATE,
            false,
        )
    }
}

/// 由 sub_611DA204 的钩子每帧调用（瓶爆子动作 → [`BURST_ACTION_TASKS`]）
///
/// 参数：`player_actor` 玩家对象
/// 参数：`variant` 子动作编号（第二参数 R1）
/// 参数：`action_state` 动作阶段（0=起手 / 1=中间 / 2=爆瓶 / 3=收尾）
///
/// 和 [`on_phial_action_frame`] 一样，只是多一个"有瓶"门槛（`require_phial = true`）：
/// `*(u8*)(player+6) == 0`（没瓶）时只允许 0x4C 执行。
pub unsafe fn on_burst_action_frame(player_actor: u32, variant: u32, action_state: u32) {
    unsafe {
        run_tasks(
            "burst",
            BURST_ACTION_TASKS,
            player_actor,
            variant,
            action_state,
            &BURST_FIRED_MASK,
            &BURST_PREV_ACTION_STATE,
            true,
        )
    }
}

/// 执行一条任务（要加新动作类型就在 match 里加分支）
///
/// 参数：`player_actor` 本次动作的玩家对象，特效直接使用其盾牌
/// 参数：`task_action` 任务内容
/// 参数：`total` 这是第几次执行（仅用于日志）
unsafe fn execute_task(player_actor: u32, task_action: &TaskAction, _total: u32) {
    match task_action {
        TaskAction::PlaySe {
            request_id,
            bank,
            attach_index,
        } => {
            let _played = unsafe { sound::play_se_ex(*request_id, *bank, *attach_index, 0) };
            /*
            error!(
                "se-src-plugin:   -> play_se_ex(request_id 0x{request_id:02X}, bank {bank}, attach_index {attach_index}) = {_played} (total {_total})"
            );
            */
        }
        TaskAction::CreateShieldEffect {
            effect_id,
            element_effect_id,
            duration_frames,
        } => {
            let _node = unsafe {
                effect::create_timed_on_shield(
                    player_actor,
                    *effect_id,
                    *element_effect_id,
                    *duration_frames,
                )
            };
            /*
            error!(
                "se-src-plugin:   -> create_shield_effect(impact 0x{effect_id:03X}, element 0x{element_effect_id:03X}, bone 0, duration {duration_frames:.2}) node=0x{_node:08X} (total {_total})"
            );
            */
        }
    }
}

/// 打印任务表（插件启动时调用）
pub fn log_task_table() {
    log_one_table("phial", ACTION_TASKS);
    log_one_table("burst", BURST_ACTION_TASKS);
}

/// 打印一张任务表
///
/// 参数：`tag` 日志前缀（`phial` / `burst`）
/// 参数：`tasks` 任务表
fn log_one_table(tag: &str, tasks: &[ActionTask]) {
    error!("se-src-plugin: {tag} task table ({} tasks)", tasks.len());
    for (index, task) in tasks.iter().enumerate() {
        match &task.action {
            TaskAction::PlaySe {
                request_id,
                bank,
                attach_index,
            } => error!(
                "se-src-plugin:   {tag} task#{index}: actionState {} variants 0x{:04X} frame {:.2} -> PlaySe(request_id 0x{request_id:02X}, bank {bank}, attach_index {attach_index})",
                task.action_state, task.variant_mask, task.trigger_frame
            ),
            TaskAction::CreateShieldEffect {
                effect_id,
                element_effect_id,
                duration_frames,
            } => error!(
                "se-src-plugin:   {tag} task#{index}: actionState {} variants 0x{:04X} frame {:.2} -> CreateShieldEffect(impact 0x{effect_id:03X}, element 0x{element_effect_id:03X}, bone 0, duration {duration_frames:.2})",
                task.action_state, task.variant_mask, task.trigger_frame
            ),
        }
    }
}

/// 累计执行任务次数
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
pub unsafe fn find_block_rule(sound_seq_ctrl: u32, command_bytes: *const u8) -> Option<usize> {
    if command_bytes.is_null() {
        return None;
    }

    let command_addr = command_bytes as u32;
    let opcode = unsafe { memory::read_u8(command_addr) };
    let scan_count = unsafe { memory::read_u8(command_addr + 2) } as u32;
    let params = command_addr + 4 + 4 * scan_count;

    let bank = unsafe { memory::read_u16(params) };
    let request_id = unsafe { memory::read_u16(params + 2) };
    let current_frame =
        unsafe { memory::read_u32(sound_seq_ctrl + constant::SOUND_SEQ_CURRENT_FRAME) };
    let prev_frame = unsafe { memory::read_u32(sound_seq_ctrl + constant::SOUND_SEQ_PREV_FRAME) };

    BLOCK_RULES.iter().position(|rule| {
        rule.opcode == opcode
            && rule.bank == bank
            && rule.request_id == request_id
            && prev_frame < rule.frame
            && rule.frame <= current_frame
    })
}

/// 打印拦截规则表（插件启动时调用）
pub fn log_block_table() {
    error!("se-src-plugin: block table ({} rules)", BLOCK_RULES.len());
    for (index, rule) in BLOCK_RULES.iter().enumerate() {
        error!(
            "se-src-plugin:   rule#{index}: frame {} opcode 0x{:02X} bank {} request_id 0x{:02X}",
            rule.frame, rule.opcode, rule.bank, rule.request_id
        );
    }
}
}
