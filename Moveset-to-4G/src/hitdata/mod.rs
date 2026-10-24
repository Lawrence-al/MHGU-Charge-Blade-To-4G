// 一次 HDP 加载：可选内嵌 asset/原版 0x4F，始终扩容并写入公共 0x50。
#[path = "../../../shared/hitdata_path.rs"]
mod cb_path;
pub mod records;
use crate::config::Config;
pub use cb_path::is_cb_path;

/// 只接受盾斧 HDP 的原版／已扩容条数；不读取其它武器的数据。
pub fn accepts(path: &[u8], count: u32, config: &Config) -> bool {
    config.needs_hitdata() && is_cb_path(path) && matches!(count, 0x50 | 0x51)
}

/// Hitdata 开关控制前80条的覆盖；公共0x50与战斗功能开关、风格无关。
pub fn apply(bytes: &mut [u8], config: &Config) {
    if config.enabled && config.hitdata {
        records::apply_original(bytes);
    }
    records::apply_extra(bytes);
}

#[cfg(target_arch = "arm")]
pub unsafe fn after_load(resource: u32, config: &Config) {
    use cb_game::{
        Game,
        resource::{HitDataResource, ResourceAllocator},
    };
    let mut resource = HitDataResource::from_address(resource);
    let path = unsafe { resource.path() };
    if !config.needs_hitdata() || !is_cb_path(path) {
        return;
    }
    let old_data = unsafe { resource.data_address() };
    let count = unsafe { resource.count() };
    if !accepts(path, count, config) {
        forge::log::prelude::error!(
            "moveset-to-4g: CB HDP count={count}, expected 80 or 81; skipped"
        );
        return;
    }
    if count == records::ORIGINAL_COUNT as u32 {
        // 仍使用原 load/unload 的 MtAllocator，保持资源释放归属。
        let allocator = unsafe { ResourceAllocator::get(Game::new(forge::mem::text_addr())) };
        let new_data = unsafe { allocator.allocate(records::EXTENDED_SIZE as u32) };
        let original =
            unsafe { core::slice::from_raw_parts(old_data as *const u8, records::ASSET_SIZE) };
        let extended =
            unsafe { core::slice::from_raw_parts_mut(new_data as *mut u8, records::EXTENDED_SIZE) };
        records::extend(original, extended);
        unsafe {
            resource.replace_data(new_data);
            allocator.free(old_data);
        }
    }
    apply(unsafe { resource.bytes_mut() }, config);
}
