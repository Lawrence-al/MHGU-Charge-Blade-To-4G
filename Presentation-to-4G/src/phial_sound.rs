//! 一二解及大解逐次瓶爆声：独立于GU/4G视觉，保留原伤害与计时。
use cb_game::{
    effect::WeaponEffectShell,
    resource::{PTS_ENTRY_SIZE, PtsEntry, SRQ_HEADER_SIZE, SRQ_RECORD_SIZE, SrqRecord},
    sound::play_common_at,
};

pub const SRQ_PATH: &[u8] = b"sound\\se\\com\\snd_com_qst";
// 原版公共SRQ最大请求为0x7D2；只追加，不覆盖187/188或其它武器的请求。
pub const IMPACT_REQUEST: u16 = 0x7D3;
pub const ELEMENT_REQUEST: u16 = 0x7D4;

/// 从同一公共SRQ复制原记录，音库、距离曲线和未知字段均不跨表借用。
pub fn requests(data: &[u8]) -> [[u8; SRQ_RECORD_SIZE]; 2] {
    let mut records = [[0; SRQ_RECORD_SIZE]; 2];
    let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    for source in data[SRQ_HEADER_SIZE..][..count * SRQ_RECORD_SIZE].chunks_exact(SRQ_RECORD_SIZE) {
        let (index, id, volume, limit) = match u16::from_le_bytes(source[..2].try_into().unwrap()) {
            0x187 => (0, IMPACT_REQUEST, -6.0, 2),
            0x188 => (1, ELEMENT_REQUEST, -4.5, 4),
            _ => continue,
        };
        records[index].copy_from_slice(source);
        let mut record = SrqRecord::new(&mut records[index]);
        record.set_request_id(id);
        // 两种瓶爆仍共用一组并发判定，但与原公共爆炸声分组分开。
        record.set_groups([220, 231, IMPACT_REQUEST]);
        record.set_volume(volume);
        record.set_concurrency_limit(limit);
        record.set_start_delay(0);
        record.set_replay_interval(10);
    }
    records
}

/// 每次真实瓶爆已独立发声，撤下原组合EFL的首声；命中附加0x4B保持原样。
pub fn patch_pts_entry(bytes: &mut [u8]) {
    debug_assert_eq!(bytes.len(), PTS_ENTRY_SIZE);
    let mut entry = PtsEntry::new(bytes);
    if matches!((entry.effect_type(), entry.effect_id(), entry.request_id(), entry.event()),
        (2, 0x208 | 0x212, 0x187, 1) | (2, 0x209 | 0x213, 0x188, 1))
    {
        entry.set_event(0);
    }
}

/// 0048874C每次瓶伤只进入一次；用当次爆点与房间排队，不依赖节点创建是否成功。
pub unsafe fn on_blast(shell: WeaponEffectShell) {
    let request = match unsafe { shell.process_type() } {
        0 | 2 => IMPACT_REQUEST,
        1 | 3 => ELEMENT_REQUEST,
        _ => return,
    };
    unsafe { play_common_at(crate::state::STATE.game(), request, shell.position_address(), shell.room()) };
}
