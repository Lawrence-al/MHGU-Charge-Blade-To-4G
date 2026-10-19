//! 超模后摇：保留原大解链，在当前玩家的表副本后追加本风格的斧竖劈派生。
use alloc::{boxed::Box, vec::Vec};
use cb_game::{
    command::{CommandTable, TableHeader},
    player::ChargeBladePlayer,
};
use core::cell::UnsafeCell;
use forge::os::sync::{LazyLock, Mutex, ScopedLock};

// Hit_func 使用 player+0x24B0 起的 0x30 字节保存逐记录输入状态。
const MAX_COMMANDS: usize = 0x30;
const AXE_CHAIN: u32 = 0x7DC;

struct ActorTable {
    player: ChargeBladePlayer,
    header: TableHeader,
    records: [u32; MAX_COMMANDS],
}

impl ActorTable {
    fn new(player: ChargeBladePlayer) -> Self {
        Self {
            player,
            header: TableHeader::ZERO,
            records: [0; MAX_COMMANDS],
        }
    }

    fn table(&self) -> CommandTable {
        CommandTable::from_address((&raw const self.header) as u32)
    }

    unsafe fn rebuild(
        &mut self,
        original: CommandTable,
        axe: CommandTable,
    ) -> Result<bool, (u32, u32)> {
        let original_count = unsafe { original.count() };
        // 序号 0 是弹刀，不加入大解；其余记录连同原输入条件和帧窗口一起复用。
        let appended = unsafe { axe.count() }.saturating_sub(1);
        if original_count > MAX_COMMANDS as u32 || appended > MAX_COMMANDS as u32 - original_count {
            return Err((original_count, appended));
        }
        if appended == 0 {
            return Ok(false);
        }
        for index in 0..original_count as usize {
            self.records[index] = unsafe { original.record(index) };
        }
        for index in 0..appended as usize {
            self.records[original_count as usize + index] = unsafe { axe.record(index + 1) };
        }
        self.header = unsafe { original.header() };
        self.header.count = original_count + appended;
        self.header.records = self.records.as_ptr() as u32;
        Ok(true)
    }
}

struct Tables {
    mutex: Mutex,
    actors: UnsafeCell<Vec<Box<ActorTable>>>,
}
unsafe impl Send for Tables {}
unsafe impl Sync for Tables {}

static TABLES: LazyLock<Tables> = LazyLock::new(|| Tables {
    mutex: Mutex::new(),
    actors: UnsafeCell::new(Vec::new()),
});

fn with_tables<R>(f: impl FnOnce(&mut Vec<Box<ActorTable>>) -> R) -> R {
    let tables = &*TABLES;
    let _lock = ScopedLock::new(&tables.mutex);
    f(unsafe { &mut *tables.actors.get() })
}

pub fn init() {
    LazyLock::force(&TABLES);
}

/// 同一玩家复用一个 Box，Vec 扩容不会移动游戏正在引用的表或数组。
/// 原 InitMoveChain 离开大解时重新写回游戏表；每次大解起手从当前资源重新合并，
/// 不缓存资源内容，不对原表或命令对象增删字段，也不向游戏交出副本所有权。
#[inline(never)]
pub unsafe fn after_chain_init(player: ChargeBladePlayer, requested_chain: u32) {
    if !matches!(requested_chain, 0x7F4 | 0x7F5 | 0x839 | 0x83A) {
        return;
    }
    if unsafe { player.equipment().weapon_type() } != 14 {
        return;
    }
    // 原初始化有拒绝切换的分支；不替错误动作挂表。
    if unsafe { player.command_chain() } != requested_chain {
        return;
    }
    let original = unsafe { player.command_table() };
    with_tables(|actors| {
        let index = actors
            .iter()
            .position(|actor| actor.player == player)
            .unwrap_or_else(|| {
                actors.push(Box::new(ActorTable::new(player)));
                actors.len() - 1
            });
        let actor = &mut actors[index];
        // 原初始化未执行／重复通知时，避免把副本作为原表再次追加。
        if original == actor.table() {
            return;
        }
        let axe = unsafe { player.command_resource().weapon_table(AXE_CHAIN) };
        match unsafe { actor.rebuild(original, axe) } {
            Ok(true) => unsafe { player.set_command_table(actor.table()) },
            Ok(false) => {}
            Err((original, appended)) => forge::log::prelude::error!(
                "moveset-to-4g: AED commands {original}+{appended} exceed {MAX_COMMANDS}; original table retained"
            ),
        }
    });
}
