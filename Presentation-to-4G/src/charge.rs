//! 黄/红充能特效的伴生声请求，供原PTS创建事件调用。
use cb_game::resource::{SRQ_RECORD_SIZE, SrqRecord};

pub const REQUEST: u16 = 0x57;

/// 从已加载的GU 0x4D复制音库/cue/音高等字段，只改新请求自己的参数。
pub fn charge_request(data: &[u8]) -> [u8; SRQ_RECORD_SIZE] {
    let mut record = [0; SRQ_RECORD_SIZE];
    let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    for source in data[cb_game::resource::SRQ_HEADER_SIZE..][..count * SRQ_RECORD_SIZE]
        .chunks_exact(SRQ_RECORD_SIZE)
    {
        if u16::from_le_bytes(source[..2].try_into().unwrap()) == 0x4D {
            record.copy_from_slice(source);
            break;
        }
    }
    let mut request = SrqRecord::new(&mut record);
    request.set_request_id(REQUEST);
    // PTS在原充能特效创建时提交私有0x57，两路音量均为0dB，启播延迟为0。
    request.set_volume(0.0);
    request.set_start_delay(0);
    record
}
