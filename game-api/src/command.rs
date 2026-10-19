//! 游戏派生资源与表视图。记录由原资源持有，插件仅合并记录指针。
use crate::memory;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandTable(u32);

/// cPlCmdTbl 的 DTI 大小为 0x1C。未知字段按原值复制，不交给游戏析构。
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableHeader {
    object: [u32; 2],
    pub count: u32,
    reserved: [u32; 2],
    pub records: u32,
    tail: u32,
}

impl TableHeader {
    pub const ZERO: Self = Self {
        object: [0; 2],
        count: 0,
        reserved: [0; 2],
        records: 0,
        tail: 0,
    };
}

const _: () = assert!(core::mem::size_of::<TableHeader>() == 0x1C);
const _: () = assert!(core::mem::offset_of!(TableHeader, count) == 8);
const _: () = assert!(core::mem::offset_of!(TableHeader, records) == 20);

impl CommandTable {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn header(self) -> TableHeader {
        let word = |offset| unsafe { memory::read_u32(self.0 + offset) };
        TableHeader {
            object: [word(0), word(4)],
            count: word(8),
            reserved: [word(12), word(16)],
            records: word(20),
            tail: word(24),
        }
    }

    pub unsafe fn count(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 8) }
    }
    pub unsafe fn record(self, index: usize) -> u32 {
        let array = unsafe { memory::read_u32(self.0 + 20) };
        unsafe { memory::read_u32(array + index as u32 * 4) }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CommandResource(u32);

impl CommandResource {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }

    /// rPlCmdTblList 的武器派生从 2000 起编号，字段与原 InitMoveChain 相同。
    pub unsafe fn weapon_table(self, chain: u32) -> CommandTable {
        let tables = unsafe { memory::read_u32(self.0 + 0x74) };
        CommandTable::from_address(unsafe { memory::read_u32(tables + (chain - 2000) * 4) })
    }
}
