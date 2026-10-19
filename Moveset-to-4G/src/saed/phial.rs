// 仅接管 uShellWp14Shell03 的瓶爆更新；复用 GU 原特效创建和瓶伤函数。
extern crate alloc;

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use forge::os::sync::{LazyLock, Mutex, ScopedLock};

use super::schedule::{self, Sequence, Step};
use cb_game::{Game, effect::Vector, shell::SaedShell};

#[derive(Clone, Copy)]
struct Record {
    shell: SaedShell,
    game: Game,
    sequence: Sequence,
    origin: [f32; 3],
    heading: [f32; 2],
    ground_y: f32,
    ground_flags: u32,
    ground_mask: u32,
}

struct Records {
    mutex: Mutex,
    entries: UnsafeCell<Vec<Record>>,
}

unsafe impl Send for Records {}
unsafe impl Sync for Records {}

static RECORDS: LazyLock<Records> = LazyLock::new(|| Records {
    mutex: Mutex::new(),
    entries: UnsafeCell::new(Vec::new()),
});

// 不持锁调用游戏函数，避免销毁 shell 时重新进入 forget 导致锁死。
fn with_records<T>(f: impl FnOnce(&mut Vec<Record>) -> T) -> T {
    let records = &*RECORDS;
    let _lock = ScopedLock::new(&records.mutex);
    f(unsafe { &mut *records.entries.get() })
}

pub fn init() {
    LazyLock::force(&RECORDS);
}

pub fn forget(shell: SaedShell) {
    with_records(|entries| entries.retain(|entry| entry.shell != shell));
}

/// 原 init 成功后保存水平朝向；首爆时重取人物位置，保留属性伤害初始化。
pub unsafe fn after_init(shell: SaedShell, game: Game) {
    let params = unsafe { shell.params() };
    let parent = unsafe { params.parent() };
    let kind = unsafe { params.kind() };
    let total = unsafe { params.phial_count() };
    let origin = unsafe { parent.position() };
    let record = Record {
        shell,
        game,
        sequence: Sequence::with_settings(kind, total, schedule::Settings::DEFAULT),
        origin,
        heading: unsafe { shell.heading(game) },
        ground_y: origin[1],
        ground_flags: unsafe { shell.ground_flags() },
        ground_mask: unsafe { shell.ground_mask() },
    };
    with_records(|entries| {
        entries.retain(|entry| entry.shell != shell);
        entries.push(record);
    });
}

/// 4G 的两次探测：上一爆高度 +300 至 -2000，失败再用 +2000 至 +300。
/// GU sub_60181440 与 4G sub_2B3610 同样在命中时返回 s0 高度、写成功字节。
unsafe fn ground_height(record: Record, position: [f32; 3]) -> f32 {
    let probe = unsafe {
        record
            .shell
            .ground_probe(record.game, record.ground_flags, record.ground_mask)
    };
    for (upper, lower) in [(300.0, -2000.0), (2000.0, 300.0)] {
        let bottom = Vector([position[0], record.ground_y + lower, position[2], 0.0]);
        let top = Vector([position[0], record.ground_y + upper, position[2], 0.0]);
        let mut hit = 0u8;
        let height = unsafe { probe.height(&bottom, &top, &mut hit) };
        if hit != 0 {
            return height;
        }
    }
    record.ground_y
}

/// 未完成初始化的 shell 返回 None，由钩子继续调用原函数。
pub unsafe fn on_frame(shell: SaedShell) -> Option<u32> {
    let game = with_records(|entries| {
        entries
            .iter()
            .find(|entry| entry.shell == shell)
            .map(|entry| entry.game)
    })?;
    // 原版父对象离场检查仍先执行，随后才访问本次瓶爆记录。
    let result = unsafe { shell.housekeeping(game) };
    if unsafe { shell.is_retired() } {
        forget(shell);
        return Some(result);
    }
    let delta = unsafe { shell.delta_frame() };
    let (record, step) = with_records(|entries| {
        let entry = entries.iter_mut().find(|entry| entry.shell == shell)?;
        let step = entry.sequence.advance(delta);
        Some((*entry, step))
    })?;
    match step {
        Step::Wait => Some(result),
        Step::Stop => {
            forget(shell);
            Some(unsafe { shell.stop() })
        }
        Step::Blast { index, remaining } => {
            let params = unsafe { shell.params() };
            let mut record = record;
            if index == 0 {
                // GU 等待期间人物仍会移动；只在首爆更新基准，之后保持这一份位置。
                let parent = unsafe { params.parent() };
                record.origin = unsafe { parent.position() };
                record.ground_y = record.origin[1];
            }
            let local = schedule::blast_with_settings(
                record.sequence.kind,
                index,
                &record.sequence.settings,
            );
            let mut position = schedule::world_xz(record.origin, record.heading, local);
            position[1] = unsafe { ground_height(record, position) };
            with_records(|entries| {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.shell == shell) {
                    entry.origin = record.origin;
                    entry.ground_y = position[1];
                }
            });
            // 特效和 hitbox 共用爆点，同时更新游戏本体及镜像坐标。
            unsafe {
                shell.set_position(position);
                shell.set_action_state(1);
                shell.create_blast_effect(game, params);
                params.set_phial_count(remaining);
            }
            // 原瓶伤函数与固定 1.35 补丁保持原样。
            let result = unsafe { shell.damage(game) };
            unsafe { shell.set_action_state(if remaining == 0 { 2 } else { 0 }) };
            Some(result)
        }
    }
}
