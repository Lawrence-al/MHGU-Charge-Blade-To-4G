// 特效命名字段与旧数组写法逐字节对照，包含未修改区域和尾部对齐字节。
use cb_game::effect::EffectCreateParams;

fn main() {
    for fill in [0u8, 0xA5, 0xFF] {
        for reference in [0x21000FD0u32, 0x47E34160] {
            let mut params = EffectCreateParams::zeroed();
            let size = core::mem::size_of::<EffectCreateParams>();
            assert_eq!(size, 0xE0);
            assert_eq!(core::mem::align_of::<EffectCreateParams>(), 16);
            unsafe { core::ptr::write_bytes(&mut params as *mut _ as *mut u8, fill, size) };
            let mut expected = vec![fill; size];
            let word = |bytes: &[u8], offset| u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap());
            expected[8..12].copy_from_slice(&3u32.to_le_bytes());
            let mask = word(&expected, 0x14) | 0x2B;
            expected[0x14..0x18].copy_from_slice(&mask.to_le_bytes());
            let bone = word(&expected, 0x50) & 0xFFFF;
            expected[0x50..0x54].copy_from_slice(&bone.to_le_bytes());
            let attach = word(&expected, 0x54) & 0xFF00FFFF;
            expected[0x54..0x58].copy_from_slice(&attach.to_le_bytes());
            let misc = word(&expected, 0x1C) | 2;
            expected[0x1C..0x20].copy_from_slice(&misc.to_le_bytes());
            expected[0xD0..0xD4].copy_from_slice(&reference.to_le_bytes());
            params.set_priority(3);
            params.attach_to(reference, 0);
            let actual = unsafe { core::slice::from_raw_parts(&params as *const _ as *const u8, size) };
            assert_eq!(actual, expected);
        }
    }
    println!("PASS: shared ARM32 effect layout/alignment and named setters match all old mask/parent/bone writes byte-for-byte");
}

