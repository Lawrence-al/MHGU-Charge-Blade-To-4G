//! 游戏特效节点与创建参数；盾牌闪电和 SAED 瓶爆共用原生参数定义。
use crate::{Game, memory, resource::Resource};

/// cProofEffectParam 的已确认字段；未知区域原样交给游戏构造／析构。
#[repr(C)]
pub struct ProofEffectParams {
    reserved_00: u32,
    mask0: u32,
    reserved_08: u32,
    misc: u32,
    position: [f32; 4],
    rotation: [f32; 4],
    scale: [f32; 4],
    effect_id: u16,
    bone: u16,
    reserved_44: [u8; 2],
    attach_mode: u8,
    reserved_47: [u8; 0x11],
    unit_mask: u32,
    surface_mask: u32,
    reserved_60: [u8; 2],
    ground_snap: u8,
    auto_surface: u8,
    reserved_64: [u8; 0x4C],
    world_position: [f32; 4],
    parent_reference: u32,
    param_script: u32,
}

/// 有效参数区 0xD8 字节；16 字节对齐后的 Rust sizeof 为 0xE0，与旧缓冲区一致。
#[repr(C, align(16))]
pub struct EffectCreateParams {
    reserved_00: [u32; 2],
    priority: u32,
    reserved_0c: u32,
    proof: ProofEffectParams,
}

impl EffectCreateParams {
    pub const fn zeroed() -> Self {
        // 数值和字节数组均接受全零位模式，没有 Rust 引用或枚举。
        unsafe { core::mem::zeroed() }
    }

    /// 参数在调用点栈上就位后再构造，保留游戏构造与析构调用顺序。
    pub unsafe fn construct(&mut self, game: Game) {
        let function: unsafe extern "C" fn(*mut Self) = unsafe { game.function(0x00040B10) };
        unsafe { function(self) };
    }

    pub unsafe fn destruct(&mut self, game: Game) {
        let function: unsafe extern "C" fn(*mut Self) = unsafe { game.function(0x00040B50) };
        unsafe { function(self) };
    }

    pub fn set_priority(&mut self, priority: u32) {
        self.priority = priority;
    }

    /// 覆盖局部位置／旋转、骨骼和附加模式；位置／旋转沿用构造结果。
    pub fn attach_to(&mut self, reference: u32, bone: u16) {
        self.proof.mask0 |= 0x2B;
        self.proof.bone = bone;
        self.proof.attach_mode = 0;
        self.proof.misc |= 2;
        self.proof.parent_reference = reference;
    }

    pub fn set_local_position(&mut self, position: [f32; 3]) {
        self.proof.mask0 |= 1;
        self.proof.position = [position[0], position[1], position[2], 0.0];
    }

    /// 原Efx_exc的misc bit0及完整参数+C0提供世界基点；mode3不再乘父对象矩阵。
    /// 后段爆炸使用固定世界位置，不能继续随挥动的剑或玩家平移。
    pub fn place_in_world(&mut self, position: [f32; 3]) {
        self.proof.misc = (self.proof.misc | 1) & !2;
        self.proof.world_position = [position[0], position[1], position[2], 0.0];
        self.proof.parent_reference = 0;
        self.proof.mask0 |= 1 | (1 << 3) | (1 << 5);
        self.proof.position = [0.0; 4];
        self.proof.attach_mode = 3;
        self.proof.bone = u16::MAX;
    }

    pub fn set_unit_mask(&mut self, mask: u32) {
        self.proof.mask0 |= 1 << 18;
        self.proof.unit_mask = mask;
    }

    /// controller的第二个筛选mask，独立于group及贴地高度。
    pub fn set_surface_mask(&mut self, mask: u32) {
        self.proof.mask0 |= 1 << 19;
        self.proof.surface_mask = mask;
    }

    /// mask0 bit24覆盖PEL自动地表；强制预览或逐爆探测时关闭，避免父缓存覆盖。
    pub fn set_auto_surface(&mut self, enabled: bool) {
        self.proof.mask0 |= 1 << 24;
        self.proof.auto_surface = u8::from(enabled);
    }

    /// 仅供不析构的借用参数副本：沿用原PEL已持有的PEP，创建器会为新core加引用。
    /// 不能对该副本调用destruct；否则会释放并未由本副本取得的资源引用。
    pub unsafe fn borrow_param_script(&mut self, script: Resource) {
        self.proof.misc |= 4;
        self.proof.param_script = script.address();
    }

    /// mask0 bit21覆盖PEL的贴地设置；为0时不再在父矩阵变换后强制改写世界Y。
    pub fn set_ground_snap(&mut self, enabled: bool) {
        self.proof.mask0 |= 1 << 21;
        self.proof.ground_snap = u8::from(enabled);
    }

    pub fn set_rotation(&mut self, rotation: [f32; 3]) {
        self.proof.mask0 |= 2;
        self.proof.rotation = [rotation[0], rotation[1], rotation[2], 0.0];
    }

    /// 原生参数以 mask0 bit30 乘上 PEL 的基础尺度；新 PEL 基础尺度为 1。
    pub fn set_scale(&mut self, scale: [f32; 3]) {
        self.proof.mask0 |= 1 << 30;
        self.proof.scale = [scale[0], scale[1], scale[2], 0.0];
    }

    /// PSL调用参数尚未合并PEL；只补原位置、附加模式和骨骼，不继承尘土尺度或group。
    pub unsafe fn inherit_pel_transform(&mut self, game: Game, handle: PelHandle, kind: u32, id: u32) -> bool {
        let Some(record) = (unsafe { handle.record(game, kind, id) }) else { return false; };
        let merge: unsafe extern "C" fn(*mut ProofEffectParams, u32, u32, u32) -> u32 =
            unsafe { game.function(0x0031C3A8) };
        let missing = !self.proof.mask0 & 0x29;
        unsafe { merge(&mut self.proof, record + 0x20, missing, 0) };
        true
    }
}

// 本类型表示 ARM32 布局，在 64 位主机检查中指针字段也必须保持 u32。
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<ProofEffectParams>() == 0xC8);
    assert!(size_of::<EffectCreateParams>() == 0xE0);
    assert!(align_of::<EffectCreateParams>() == 16);
    assert!(offset_of!(EffectCreateParams, priority) == 0x08);
    assert!(offset_of!(EffectCreateParams, proof) == 0x10);
    assert!(offset_of!(ProofEffectParams, mask0) == 0x04);
    assert!(offset_of!(ProofEffectParams, misc) == 0x0C);
    assert!(offset_of!(ProofEffectParams, position) == 0x10);
    assert!(offset_of!(ProofEffectParams, unit_mask) == 0x58);
    assert!(offset_of!(ProofEffectParams, surface_mask) == 0x5C);
    assert!(offset_of!(ProofEffectParams, ground_snap) == 0x62);
    assert!(offset_of!(ProofEffectParams, auto_surface) == 0x63);
    assert!(offset_of!(ProofEffectParams, world_position) == 0xB0);
    assert!(offset_of!(ProofEffectParams, bone) == 0x42);
    assert!(offset_of!(ProofEffectParams, attach_mode) == 0x46);
    assert!(offset_of!(ProofEffectParams, parent_reference) == 0xC0);
    assert!(offset_of!(ProofEffectParams, param_script) == 0xC4);
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectNode(u32);

impl EffectNode {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn effect_id(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x1B4) }
    }

    pub unsafe fn is_finishing(self) -> bool {
        unsafe { memory::read_u8(self.0 + 0x30) == 2 }
    }

    pub unsafe fn delta_frame(self) -> f32 {
        unsafe { memory::read_f32(self.0 + 0x1C) }
    }

    /// 在state30建立EFL child之前写入本次爆点的分类结果；不改节点坐标。
    pub unsafe fn set_surface_mask(self, mask: u32) {
        unsafe {
            memory::write_u32(self.0 + 0xDC, mask);
            let flags = memory::read_u32(self.0 + 0x84);
            memory::write_u32(self.0 + 0x84, flags | (1 << 19));
        }
    }

    /// 第二参数 0 走 PSL 同样的收尾路径，停止生成并等待已有粒子结束。
    pub unsafe fn stop(self, game: Game) {
        let function: unsafe extern "C" fn(u32, u32) -> u32 = unsafe { game.function(0x0032A400) };
        unsafe { function(self.0, 0) };
    }

    /// 参数1立即执行原生清理虚函数，再置state=2；仅供预览切换时清掉上一份。
    pub unsafe fn stop_immediately(self, game: Game) {
        let function: unsafe extern "C" fn(u32, u32) -> u32 = unsafe { game.function(0x0032A400) };
        unsafe { function(self.0, 1) };
    }

    /// 新 PEL 的 ID 与原 PTS 的查询 ID 不同；沿用原声源和原请求，不另播一份 SE。
    pub unsafe fn play_companion(self, game: Game, sound_table: u32, original_id: u32) {
        let create: unsafe extern "C" fn(u32, u32, u32) -> u32 =
            unsafe { game.function(0x00042F08) };
        let key = self.0 + 0x1B4;
        let custom_id = unsafe { memory::read_u32(key) };
        unsafe {
            memory::write_u32(key, original_id);
            create(self.0, sound_table, 2);
            memory::write_u32(key, custom_id);
        }
    }
}

/// rProofEffectList 先注册到 sProofEffect，再以 {…, index} 句柄创建节点。
/// 注册会另持有资源引用；常驻特效包随 NRO 保持到进程结束。
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PelHandle(u32);

impl PelHandle {
    pub const fn from_address(address: u32) -> Self { Self(address) }
    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn registered_index(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 4) }
    }

    /// 已注册PEL中按组及ID查询加载后的记录；同一数字ID在不同组可以重复。
    pub unsafe fn record(self, game: Game, kind: u32, id: u32) -> Option<u32> {
        let manager = unsafe { memory::read_u32(game.address(0x0188A0D4)) };
        let entries = unsafe { memory::read_u32(manager + 0x60) };
        let pel = unsafe { memory::read_u32(entries + self.registered_index() * 4) };
        let count = unsafe { memory::read_u32(pel + 0x68 + kind * 0x14) };
        let records = unsafe { memory::read_u32(pel + 0x74 + kind * 0x14) };
        (0..count).find_map(|index| {
            let record = unsafe { memory::read_u32(records + index * 4) };
            (unsafe { memory::read_u16(record + 0x50) } as u32 == id).then_some(record)
        })
    }

    pub unsafe fn registry_usage(game: Game) -> [u32; 2] {
        let manager = unsafe { memory::read_u32(game.address(0x0188A0D4)) };
        let count = unsafe { memory::read_u32(manager + 0x5C) };
        let entries = unsafe { memory::read_u32(manager + 0x60) };
        let used = (0..count)
            .filter(|index| unsafe { memory::read_u32(entries + index * 4) } != 0)
            .count() as u32;
        [count, used]
    }

    pub unsafe fn register(game: Game, resource: Resource) -> Self {
        let register: unsafe extern "C" fn(u32, u32) -> u32 =
            unsafe { game.function(0x003268D8) };
        let manager = unsafe { memory::read_u32(game.address(0x0188A0D4)) };
        Self(unsafe { register(manager, resource.address()) })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct WeaponEffectShell(u32);

impl WeaponEffectShell {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }

    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn create_defined(
        self,
        game: Game,
        definition: *const u32,
        params: &mut EffectCreateParams,
    ) -> EffectNode {
        let create: unsafe extern "C" fn(u32, *const u32, &mut EffectCreateParams) -> u32 =
            unsafe { game.function(0x004A1E50) };
        EffectNode(unsafe { create(self.0, definition, params) })
    }

    pub unsafe fn create_unique(
        self,
        game: Game,
        id: u16,
        params: &EffectCreateParams,
    ) -> EffectNode {
        let pel = PelHandle(unsafe { memory::read_u32(self.0 + 0x1388) });
        unsafe { self.create_from_pel(game, pel, id, params) }
    }

    pub unsafe fn create_from_pel(
        self,
        game: Game,
        pel: PelHandle,
        id: u16,
        params: &EffectCreateParams,
    ) -> EffectNode {
        let create: unsafe extern "C" fn(
            u32,
            u32,
            u32,
            u32,
            *const EffectCreateParams,
            u32,
        ) -> u32 = unsafe { game.function(0x004A1E08) };
        // UNIQUE，sound_table=0：主动创建不另行提交伴生音效。
        EffectNode(unsafe { create(self.0, pel.0, 2, id as u32, params, 0) })
    }

    /// CreateShell02 的 setup 类型：0/1=普通瓶爆，2/3=AED 命中瓶爆。
    pub unsafe fn process_type(self) -> u8 {
        let setup = unsafe { memory::read_u32(self.0 + 0x136C) };
        unsafe { memory::read_u8(setup + 8) }
    }

    pub unsafe fn owner(self) -> crate::player::ChargeBladePlayer {
        let setup = unsafe { memory::read_u32(self.0 + 0x136C) };
        crate::player::ChargeBladePlayer::from_address(unsafe { memory::read_u32(setup + 0x0C) })
    }

    /// CreateShell02的a3：0为一二解，1包括盾突；只替换前者的延迟瓶爆。
    pub unsafe fn selector(self) -> u8 {
        unsafe { memory::read_u8(self.0 + 0x15C4) }
    }

    pub unsafe fn remaining_phials(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x15C0) }
    }

    pub unsafe fn action_state(self) -> u8 {
        unsafe { memory::read_u8(self.0 + 5) }
    }

    /// 首层附着由原版保存/停止；此槽允许为空，不是必须有效的对象指针。
    pub unsafe fn attachment(self) -> Option<EffectNode> {
        let address = unsafe { memory::read_u32(self.0 + 0x15DC) };
        (address != 0).then_some(EffectNode::from_address(address))
    }

    /// 004880B8对type1/3调用011E00C8后保存的实际攻击元素位。
    pub unsafe fn element_flags(self) -> u64 {
        unsafe {
            u64::from(memory::read_u32(self.0 + 0x15C8))
                | (u64::from(memory::read_u32(self.0 + 0x15CC)) << 32)
        }
    }

    /// 定义中的list指向当前shell已注册PEL；返回匹配记录借用的已加载PEP资源。
    pub unsafe fn param_script(self, game: Game, list: u32, id: u32) -> Option<Resource> {
        let handle = PelHandle(unsafe { memory::read_u32(self.0 + 0x1388 + list * 4) });
        // create_defined使用UNIQUE组，list不是PEL内部的组号。
        unsafe { handle.record(game, 2, id) }.map(|record|
            Resource::from_address(unsafe { memory::read_u32(record + 0x18) }))
    }

    pub unsafe fn sound_table(self, group: u32) -> u32 {
        let resource = unsafe { memory::read_u32(self.0 + 0x1384) };
        unsafe { memory::read_u32(resource + 0x98 + group * 4) }
    }
}

/// GU 地面查询使用 16 字节向量；第四项保留为 0。
#[repr(C, align(16))]
pub struct Vector(pub [f32; 4]);
