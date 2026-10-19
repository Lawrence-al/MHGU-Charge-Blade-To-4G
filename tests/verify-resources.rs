// 用 dump 原版和 asset 改版核对实际 Rust 修改函数，不运行游戏或 Forge。
#[allow(dead_code)]
mod resource {
    include!("../Presentation-to-4G/src/resource.rs");
}

use std::{env, fs};

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn verify_srq(original: &[u8], asset: &[u8]) {
    assert_eq!(&original[..4], b"SRQR");
    assert_eq!(u32_at(original, 4), 5);
    assert_eq!(u32_at(original, 8), 49);
    let mut patched = original.to_vec();
    resource::patch_srq_data(&mut patched);
    assert_eq!(patched, asset, "SRQ 修改后的完整文件必须等于 asset");
    resource::patch_srq_data(&mut patched);
    assert_eq!(patched, asset, "对已修改 SRQ 再应用一次必须保持原值");
    let changes = original.iter().zip(asset).filter(|(a, b)| a != b).count();
    println!("PASS: SRQ 全文件 {} 字节一致，复现 {} 个差异字节；重复应用一致", patched.len(), changes);
}

fn verify_pts(original: &[u8], asset: &[u8]) {
    // 本次两份 XFS 的 63 条固定字段记录；与运行时对象的 24 字节结构分别解码。
    assert_eq!(&original[..4], b"XFS\0");
    assert_eq!(original.len(), 4671);
    let mut patched = original.to_vec();
    let mut count = 0;
    let mut changed = 0;
    for uid_offset in (0x38C..original.len()).step_by(0x3C) {
        if uid_offset + 43 > original.len() {
            break;
        }
        // 每个字段前的数组计数由序列化器写入，避免把文件布局误当对象布局。
        assert_eq!(u32_at(original, uid_offset + 2), 1);
        assert_eq!(u32_at(original, uid_offset + 18), 1);
        let mut entry = [0xA5; resource::PTS_ENTRY_SIZE];
        entry[4..6].copy_from_slice(&(count as u16).to_le_bytes());
        entry[6] = original[uid_offset - 5];
        entry[8..10].copy_from_slice(&original[uid_offset..uid_offset + 2]);
        entry[10] = original[uid_offset + 6];
        entry[11] = original[uid_offset + 11];
        entry[12..14].copy_from_slice(&original[uid_offset + 16..uid_offset + 18]);
        entry[14] = original[uid_offset + 22];
        entry[15] = original[uid_offset + 27];
        entry[16] = original[uid_offset + 32];
        entry[17] = original[uid_offset + 37];
        entry[18] = original[uid_offset + 42];
        let before = entry;
        resource::patch_pts_entry(&mut entry);
        for index in 0..entry.len() {
            if !(12..=14).contains(&index) {
                assert_eq!(entry[index], before[index], "PTS 对象中其它字段被修改");
            }
        }
        let once = entry;
        resource::patch_pts_entry(&mut entry);
        assert_eq!(entry, once, "对已修改 PTS 再应用一次必须保持原值");
        patched[uid_offset + 16..uid_offset + 18].copy_from_slice(&entry[12..14]);
        patched[uid_offset + 22] = entry[14];
        changed += usize::from(before != entry);
        count += 1;
    }
    assert_eq!(count, 63);
    assert_eq!(changed, 13);
    assert_eq!(patched, asset, "PTS 修改后重新映射的完整文件必须等于 asset");
    println!("PASS: PTS {} 条运行时记录，修改 {} 条；完整 {} 字节等于 asset，其它字段及重复应用一致", count, changed, patched.len());
}

fn verify_paths() {
    for expected in [resource::SRQ_PATH, resource::PTS_PATH] {
        assert!(resource::path_matches(expected, expected));
        let mut alternate = expected.to_ascii_uppercase();
        for byte in &mut alternate {
            if *byte == b'\\' {
                *byte = b'/';
            }
        }
        alternate.extend_from_slice(b"\0ignored");
        assert!(resource::path_matches(&alternate, expected));
        let mut different = expected.to_vec();
        different.push(b'x');
        assert!(!resource::path_matches(&different, expected));
    }
    assert!(resource::path_matches(b"sound/se/pl/wp/gaxe/com/snd_wp_gaxe_com.SRQ", resource::SRQ_PATH));
    assert!(resource::path_matches(b"sound/pts/pl/w14_000.pts", resource::PTS_PATH));
    assert!(!resource::path_matches(b"sound/pts/pl/w13_000", resource::PTS_PATH));
    assert!(!resource::path_matches(b"sound/se/pl/wp/gaxe/com/snd_wp_gaxe_other", resource::SRQ_PATH));
    println!("PASS: 资源路径过滤、分隔符、大小写及扩展名");
}

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 5, "参数：原版 SRQ、asset SRQ、原版 PTS、asset PTS");
    let files: Vec<_> = args[1..].iter().map(|path| fs::read(path).unwrap()).collect();
    // request 字段确实出现在预期文件位置。
    assert_eq!(u16_at(&files[0], 0x97C), 0x4A);
    assert_eq!(u16_at(&files[0], 0xEA4), 0x54);
    verify_srq(&files[0], &files[1]);
    verify_pts(&files[2], &files[3]);
    verify_paths();
}
