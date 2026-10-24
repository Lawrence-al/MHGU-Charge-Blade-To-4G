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
    for mask in [1u32, 2, 0xFFFF_FFFF] {
        let mut params = EffectCreateParams::zeroed();
        params.set_local_position([30.0, 0.0, 250.0]);
        params.set_rotation([0.0; 3]);
        params.set_scale([1.0; 3]);
        params.set_unit_mask(mask);
        let bytes = unsafe { core::slice::from_raw_parts(&params as *const _ as *const u8, 0xE0) };
        let word = |offset| u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap());
        assert_eq!(word(0x14), 3 | (1 << 18) | (1 << 30));
        assert_eq!(word(0x68), mask);
        for (index, value) in [30.0f32, 0.0, 250.0, 0.0].into_iter().enumerate() {
            assert_eq!(word(0x20 + index * 4), value.to_bits());
        }
        for index in 0..3 { assert_eq!(word(0x40 + index * 4), 1.0f32.to_bits()); }
    }
    println!("PASS: shared ARM32 effect layout/alignment and named setters match all old mask/parent/bone writes byte-for-byte");
    for enabled in [false, true] {
        let mut params = EffectCreateParams::zeroed();
        params.set_ground_snap(enabled);
        let bytes = unsafe { core::slice::from_raw_parts(&params as *const _ as *const u8, 0xE0) };
        assert_eq!(u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()), 1 << 21);
        assert_eq!(bytes[0x72], u8::from(enabled));
        assert_eq!(bytes[0x71], 0);
        assert_eq!(bytes[0x73], 0);
    }
}

