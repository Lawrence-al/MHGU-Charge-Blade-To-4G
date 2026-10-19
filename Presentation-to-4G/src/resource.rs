// resource.rs
// 将两份音效 asset 的改动应用到游戏加载的资源，避免另行覆盖 SRQ / PTS 文件。

#[cfg(not(target_arch = "arm"))]
pub use cb_game::resource::PTS_ENTRY_SIZE;
use cb_game::resource::{PtsEntry, SrqRecord};
pub use cb_game::resource::{SRQ_HEADER_SIZE, SRQ_RECORD_SIZE};

pub const SRQ_PATH: &[u8] = b"sound\\se\\pl\\wp\\gaxe\\com\\snd_wp_gaxe_com";
pub const PTS_PATH: &[u8] = b"sound\\pts\\pl\\w14_000";

// 与本次 asset 一致的参数快照；构建和运行都不依赖 asset 文件的位置。
#[cfg(not(target_arch = "arm"))]
const SRQ_4A_VOLUME: f32 = -1.9;
const SRQ_4A_REPLAY_INTERVAL: u16 = 0;
const SRQ_54: [u8; SRQ_RECORD_SIZE] = [
    0x54, 0x00, 0x4A, 0x7F, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x00, 0xC8, 0x00, 0x6E, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00,
    0xFF, 0xFF, 0x00, 0x00, 0x9A, 0x99, 0x19, 0xC0, 0x9A, 0x99, 0x19, 0xC0, 0x58, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xC0, 0xC2, 0xFF, 0xFF, 0xFF, 0xFF, 0x64, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
    0x00, 0x00, 0x00, 0x00,
];

fn normalize_path_byte(value: u8) -> u8 {
    if value == b'/' {
        b'\\'
    } else {
        value.to_ascii_lowercase()
    }
}

/// MtResource::mPath 是内嵌的 64 字节路径，允许两种分隔符及资源扩展名。
pub fn path_matches(path: &[u8], expected: &[u8]) -> bool {
    let end = path
        .iter()
        .position(|&value| value == 0)
        .unwrap_or(path.len());
    let path = &path[..end];
    if path.len() < expected.len() {
        return false;
    }
    for (actual, expected) in path.iter().zip(expected) {
        if normalize_path_byte(*actual) != normalize_path_byte(*expected) {
            return false;
        }
    }
    let suffix = &path[expected.len()..];
    suffix.is_empty()
        || suffix.eq_ignore_ascii_case(b".srq")
        || suffix.eq_ignore_ascii_case(b".srqr")
        || suffix.eq_ignore_ascii_case(b".pts")
}

/// 只在二进制 SRQR 刚读入、尚未绑定音库及转换内部索引时调用。
#[cfg(not(target_arch = "arm"))]
pub fn patch_srq_data(data: &mut [u8]) {
    patch_srq_data_with_volume(data, SRQ_4A_VOLUME);
}

pub fn patch_srq_data_with_volume(data: &mut [u8], volume: f32) {
    let count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    for record in data[SRQ_HEADER_SIZE..SRQ_HEADER_SIZE + count * SRQ_RECORD_SIZE]
        .chunks_exact_mut(SRQ_RECORD_SIZE)
    {
        let mut record = SrqRecord::new(record);
        match record.request_id() {
            0x4A => {
                // +0x24/+0x28：音量；+0x34：同组重播间隔。
                record.set_volume(volume);
                record.set_replay_interval(SRQ_4A_REPLAY_INTERVAL);
            }
            0x54 => {
                // asset 把停止请求改为 cue 0x4A 的播放请求，含音库索引及所有参数。
                // 此时 +0x58/+0x80 仍是文件中的未绑定值，随后由原函数建立引用。
                record.replace(&SRQ_54);
            }
            _ => {}
        }
    }
}

/// 修改反序列化后的 cFestaPelTiedSe（24 字节），不使用 XFS 文件偏移。
pub fn patch_pts_entry(entry: &mut [u8]) {
    let mut entry = PtsEntry::new(entry);
    let effect_type = entry.effect_type();
    let unique_id = entry.effect_id();
    let request = entry.request_id();
    let event = entry.event();

    match (effect_type, unique_id, request, event) {
        (1, 0x226, 0x4A, 1) | (1, 0x226, 0x56, 3) => {
            entry.set_event(0);
        }
        (2, 0x1F4, 0x54, 3) => {
            // 与 asset 一致：请求号改为 0，保留结束事件类型 3。
            entry.set_request_id(0);
        }
        (1, 0x258 | 0x259 | 0x262 | 0x263, 0x48, 1)
        | (2, 0x258 | 0x259 | 0x25D | 0x25E | 0x262 | 0x263, 0x48, 1) => {
            entry.set_request_id(0x47);
        }
        _ => {}
    }
}

/// SRQR 绑定前只修改对应武器路径的原始记录。
pub unsafe fn before_srq_bind(address: u32, volume: f32) {
    let mut resource = cb_game::resource::SoundRequestResource::from_address(address);
    if !path_matches(unsafe { resource.path() }, SRQ_PATH) {
        return;
    }
    patch_srq_data_with_volume(unsafe { resource.bytes_mut() }, volume);
}

/// PTS load 成功后只修改对应路径的运行时记录。
pub unsafe fn after_pts_load(address: u32) {
    let resource = cb_game::resource::PtsResource::from_address(address);
    if !path_matches(unsafe { resource.path() }, PTS_PATH) {
        return;
    }
    unsafe { resource.for_each_entry(patch_pts_entry) };
}
