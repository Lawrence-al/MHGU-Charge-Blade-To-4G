//! 加载后的资源对象与记录视图；SRQR 文件记录和 PTS 运行时对象分别定义。
use crate::{Game, memory};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Resource(u32);

impl Resource {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn load_flags(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x50) }
    }

    pub unsafe fn archive_entry_count(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x68) }
    }

    pub unsafe fn pel_group_counts(self) -> [u32; 3] {
        core::array::from_fn(|group| unsafe {
            memory::read_u32(self.0 + 0x68 + group as u32 * 20)
        })
    }

    pub unsafe fn release(self, game: Game) {
        let release: unsafe extern "C" fn(u32) = unsafe { game.function(0x00887660) };
        unsafe { release(self.0) };
    }
    /// MtResource 内嵌 64 字节路径；沿用 load 成功后的有效资源前提。
    pub unsafe fn path(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts((self.0 + 8) as *const u8, 64) }
    }
}

/// rArchive 的内存加载路径。资源数据由原游戏分配，输入 ARC 保持只读。
/// 使用方必须持有返回的 ARC 引用，直到其中所有依赖均不再使用。
pub unsafe fn load_embedded_archive(
    game: Game,
    path: &core::ffi::CStr,
    bytes: &'static [u8],
) -> Resource {
    // MtMemoryStream 的已用字段到 +0x18；多保留一字，构造后仍使用原游戏析构。
    let mut stream = [0u32; 8];
    let construct: unsafe extern "C" fn(*mut u32, *const u8, u32, u32) -> u32 =
        unsafe { game.function(0x008385E8) };
    let destruct: unsafe extern "C" fn(*mut u32) -> u32 =
        unsafe { game.function(0x00838634) };
    let load: unsafe extern "C" fn(u32, *mut u32, u32, *const core::ffi::c_char, u32) -> u32 =
        unsafe { game.function(0x00BCA65C) };
    let manager = unsafe { memory::read_u32(game.address(0x02126A64)) };
    unsafe { construct(stream.as_mut_ptr(), bytes.as_ptr(), bytes.len() as u32, 1) };
    // 与原 DLC 内存 ARC 相同：DTI=rArchive，质量档位 4；同步 load/loadEnd。
    let archive = unsafe {
        load(manager, stream.as_mut_ptr(), game.address(0x02124DF0), path.as_ptr(), 4)
    };
    unsafe { destruct(stream.as_mut_ptr()) };
    Resource(archive)
}

/// 只在已加载 ARC 的缓存里查询 PEL，不尝试从 romfs 查找同名裸文件。
pub unsafe fn find_loaded_pel(game: Game, path: &core::ffi::CStr) -> Resource {
    let manager = unsafe { memory::read_u32(game.address(0x02126A64)) };
    let load: unsafe extern "C" fn(u32, u32, *const core::ffi::c_char, u32) -> u32 =
        unsafe { memory::method(manager, 0x30) };
    Resource(unsafe { load(manager, game.address(0x01889FDC), path.as_ptr(), 0x80) })
}

pub const SRQ_HEADER_SIZE: usize = 0x34;
pub const SRQ_RECORD_SIZE: usize = 0x84;
pub const PTS_ENTRY_SIZE: usize = 0x18;
pub const HDP_HEADER_SIZE: usize = 0x10;
pub const HDP_RECORD_SIZE: usize = 0x38;
pub const HDP_ORIGINAL_COUNT: usize = 0x50;

/// SRQR 的原始文件缓冲区；只在绑定音库之前使用。
pub struct SoundRequestResource(Resource);

impl SoundRequestResource {
    pub const fn from_address(address: u32) -> Self {
        Self(Resource(address))
    }
    pub unsafe fn path(&self) -> &[u8] {
        unsafe { self.0.path() }
    }
    pub unsafe fn bytes_mut(&mut self) -> &mut [u8] {
        let data = unsafe { memory::read_u32(self.0.0 + 0x64) };
        let count = unsafe { memory::read_u32(data + 8) } as usize;
        unsafe {
            core::slice::from_raw_parts_mut(
                data as *mut u8,
                SRQ_HEADER_SIZE + count * SRQ_RECORD_SIZE,
            )
        }
    }
}

/// XFS 解析后的 cFestaPelTiedSe 指针数组，不是序列化 PTS 文件布局。
pub struct PtsResource(Resource);

impl PtsResource {
    pub const fn from_address(address: u32) -> Self {
        Self(Resource(address))
    }
    pub unsafe fn path(&self) -> &[u8] {
        unsafe { self.0.path() }
    }
    pub unsafe fn for_each_entry(self, mut apply: impl FnMut(&mut [u8])) {
        let count = unsafe { memory::read_u32(self.0.0 + 0x68) };
        let entries = unsafe { memory::read_u32(self.0.0 + 0x74) };
        for index in 0..count {
            let entry = unsafe { memory::read_u32(entries + index * 4) };
            apply(unsafe { core::slice::from_raw_parts_mut(entry as *mut u8, PTS_ENTRY_SIZE) });
        }
    }
}

/// 用字节视图访问文件记录，保留未知字节和非对齐字段。
pub struct SrqRecord<'a>(&'a mut [u8]);

impl<'a> SrqRecord<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self(bytes)
    }
    pub fn request_id(&self) -> u16 {
        u16::from_le_bytes(self.0[0..2].try_into().unwrap())
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.0[0x24..0x28].copy_from_slice(&volume.to_le_bytes());
        self.0[0x28..0x2C].copy_from_slice(&volume.to_le_bytes());
    }
    pub fn set_replay_interval(&mut self, interval: u16) {
        self.0[0x34..0x36].copy_from_slice(&interval.to_le_bytes());
    }
    pub fn replace(&mut self, snapshot: &[u8; SRQ_RECORD_SIZE]) {
        self.0.copy_from_slice(snapshot);
    }
}

pub struct PtsEntry<'a>(&'a mut [u8]);

impl<'a> PtsEntry<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self(bytes)
    }
    pub fn effect_type(&self) -> u8 {
        self.0[6]
    }
    pub fn effect_id(&self) -> u16 {
        u16::from_le_bytes(self.0[8..10].try_into().unwrap())
    }
    pub fn request_id(&self) -> u16 {
        u16::from_le_bytes(self.0[12..14].try_into().unwrap())
    }
    pub fn event(&self) -> u8 {
        self.0[14]
    }
    pub fn set_request_id(&mut self, request: u16) {
        self.0[12..14].copy_from_slice(&request.to_le_bytes());
    }
    pub fn set_event(&mut self, event: u8) {
        self.0[14] = event;
    }
}

/// rHitData 中的 HDP 缓冲区；所有者和释放器仍为原游戏 MtAllocator。
pub struct HitDataResource(Resource);

impl HitDataResource {
    pub const fn from_address(address: u32) -> Self {
        Self(Resource(address))
    }
    pub unsafe fn path(&self) -> &[u8] {
        unsafe { self.0.path() }
    }
    pub unsafe fn data_address(&self) -> u32 {
        unsafe { memory::load_u32(self.0.0 + 0x64) }
    }
    pub unsafe fn count(&self) -> u32 {
        unsafe { memory::load_u32(self.data_address() + 8) }
    }
    pub unsafe fn replace_data(&mut self, address: u32) {
        unsafe {
            memory::write_u32(self.0.0 + 0x64, address);
            memory::write_u32(self.0.0 + 0x68, address + HDP_HEADER_SIZE as u32);
        }
    }
    pub unsafe fn bytes_mut(&mut self) -> &mut [u8] {
        let data = unsafe { self.data_address() };
        let count = unsafe { memory::load_u32(data + 8) } as usize;
        unsafe {
            core::slice::from_raw_parts_mut(
                data as *mut u8,
                HDP_HEADER_SIZE + count * HDP_RECORD_SIZE,
            )
        }
    }
}

pub struct ResourceAllocator(u32);

impl ResourceAllocator {
    pub unsafe fn get(game: Game) -> Self {
        let get: unsafe extern "C" fn(u32) -> u32 = unsafe { game.function(0x007AA47C) };
        Self(unsafe { get(game.address(0x0188C110)) })
    }
    pub unsafe fn allocate(&self, size: u32) -> u32 {
        let allocate: unsafe extern "C" fn(u32, u32, u32) -> u32 =
            unsafe { memory::method(self.0, 0x1C) };
        unsafe { allocate(self.0, size, 16) }
    }
    pub unsafe fn free(&self, address: u32) {
        let free: unsafe extern "C" fn(u32, u32) = unsafe { memory::method(self.0, 0x34) };
        unsafe { free(self.0, address) };
    }
}
