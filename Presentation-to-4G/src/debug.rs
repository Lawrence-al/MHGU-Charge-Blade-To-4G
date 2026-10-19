//! 调试输出集中管理；仅启用 debug-logs feature 时编译并安装日志钩子。

use ::core::sync::atomic::{AtomicU32, Ordering};

use forge::log::prelude::error;

use crate::{
    action,
    state::STATE,
    tasks::{ACTION_TASKS, ActionTask, BLOCK_RULES, BURST_ACTION_TASKS, TaskAction},
};
use cb_game::memory;

/// 测试结束后改成 false，再编译，即可关闭这两个日志钩子。
const ENABLED: bool = true;

static CREATE_COUNT: AtomicU32 = AtomicU32::new(0);

/// sub_6032AA44：PEL 粒子特效实例创建后的参数设置。
/// 第四个参数已经合并 PEL 默认值、调用者覆盖值与参数脚本。
#[forge::hook(offset = 0x0032AA44)]
fn hook_create_particle(node: u32, instance: u32, resource: u32, params: u32) -> u32 {
    let result = unsafe { original!(node, instance, resource, params) };
    unsafe { log_created("particle", node, instance, params) };
    result
}

/// sub_6032AC64：PEL 模型特效的同一条创建链。
#[forge::hook(offset = 0x0032AC64)]
fn hook_create_model(node: u32, instance: u32, resource: u32, params: u32) -> u32 {
    let result = unsafe { original!(node, instance, resource, params) };
    unsafe { log_created("model", node, instance, params) };
    result
}

/// 原创建函数已成功取到 PEL 条目及其资源，直接沿用该条目的有效指针。
/// params 是带 0x30 字节前缀的运行时参数块，偏移不能当成 PEL 文件偏移。
unsafe fn log_created(kind: &str, node: u32, instance: u32, params: u32) {
    let pel = unsafe { memory::read_u32(node + 0x34) };
    let group = unsafe { memory::read_u32(node + 0x40) };
    let entry_index = unsafe { memory::read_u32(node + 0x44) };
    let entries = unsafe { memory::read_u32(pel + 0x74 + 20 * group) };
    let entry = unsafe { memory::read_u32(entries + 4 * entry_index) };

    // MtString：+4 为长度，+8 开始是路径正文；只打印盾斧的特效资源。
    let path_string = unsafe { memory::read_u32(entry + 0x14) };
    let path_len = unsafe { memory::read_u32(path_string + 4) } as usize;
    let path_bytes =
        unsafe { ::core::slice::from_raw_parts((path_string + 8) as *const u8, path_len) };
    if !path_bytes.starts_with(b"effect\\pl\\w14\\") {
        return;
    }
    let path = unsafe { ::core::str::from_utf8_unchecked(path_bytes) };

    let effect_id = unsafe { memory::read_u16(params + 0x70) };
    let bone = unsafe { memory::read_u16(params + 0x72) } as i16;
    let attach_mode = unsafe { memory::read_u8(params + 0x76) };
    let bind_mode = unsafe { memory::read_u32(params) };
    let parent_ref = unsafe { memory::read_u32(params + 0xF0) };
    // 实例 +0x30 是 mpParent。粒子挂骨骼时为实际父对象；模型仍需看 parentRef。
    let parent = unsafe { memory::read_u32(instance + 0x30) };
    let position = unsafe {
        [
            memory::read_f32(params + 0x40),
            memory::read_f32(params + 0x44),
            memory::read_f32(params + 0x48),
        ]
    };
    let rotation = unsafe {
        [
            memory::read_f32(params + 0x50),
            memory::read_f32(params + 0x54),
            memory::read_f32(params + 0x58),
        ]
    };
    let group_name = match group {
        0 => "RULED",
        1 => "SEQUENCE",
        2 => "UNIQUE",
        _ => "UNKNOWN",
    };
    let count = CREATE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

    error!(
        "[effect-id] #{count} effectId=0x{effect_id:03X} group={group_name} kind={kind} bone={bone} attachMode={attach_mode} bindMode={bind_mode} parentRef=0x{parent_ref:08X} parent=0x{parent:08X}"
    );
    error!(
        "[effect-id] #{count} pos=({:.2},{:.2},{:.2}) rot=({:.2},{:.2},{:.2}) path={path} pel=0x{pel:08X} entry={entry_index} node=0x{node:08X} instance=0x{instance:08X}",
        position[0], position[1], position[2], rotation[0], rotation[1], rotation[2],
    );
}

pub fn enable(game_base: u32) {
    log_task_table();
    log_block_table();
    error!(
        "presentation-to-4g: counters dispatch={} play={} task={}",
        STATE.dispatch_count(),
        STATE.play_count(),
        action::get_task_run_count()
    );
    if ENABLED {
        forge::install_hook!(game_base, hook_create_particle);
        forge::install_hook!(game_base, hook_create_model);
        error!("[effect-id] enabled: w14 PEL creation log (particle + model)");
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
    error!(
        "presentation-to-4g: {tag} task table ({} tasks)",
        tasks.len()
    );
    for (index, task) in tasks.iter().enumerate() {
        match &task.action {
            TaskAction::PlaySe {
                request_id,
                bank,
                attach_index,
            } => error!(
                "presentation-to-4g:   {tag} task#{index}: actionState {} variants 0x{:04X} frame {:.2} -> PlaySe(request_id 0x{request_id:02X}, bank {bank}, attach_index {attach_index})",
                task.action_state, task.variant_mask, task.trigger_frame
            ),
            TaskAction::CreateShieldEffect {
                effect_id,
                element_effect_id,
                duration_frames,
            } => error!(
                "presentation-to-4g:   {tag} task#{index}: actionState {} variants 0x{:04X} frame {:.2} -> CreateShieldEffect(impact 0x{effect_id:03X}, element 0x{element_effect_id:03X}, bone 0, duration {duration_frames:.2})",
                task.action_state, task.variant_mask, task.trigger_frame
            ),
        }
    }
}

/// 打印拦截规则表（插件启动时调用）
pub fn log_block_table() {
    error!(
        "presentation-to-4g: block table ({} rules)",
        BLOCK_RULES.len()
    );
    for (index, rule) in BLOCK_RULES.iter().enumerate() {
        error!(
            "presentation-to-4g:   rule#{index}: frame {} opcode 0x{:02X} bank {} request_id 0x{:02X}",
            rule.frame, rule.opcode, rule.bank, rule.request_id
        );
    }
}
