// 两个 HDP 插件共用盾斧资源路径匹配，允许原版两种目录和 .hdp 后缀。
pub fn is_cb_path(path: &[u8]) -> bool {
    let end = path
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(path.len());
    let path = &path[..end];
    [
        b"player\\hit\\pl_we14_hitdata".as_slice(),
        b"hit\\pl_we14_hitdata".as_slice(),
    ]
    .iter()
    .any(|expected| {
        path.len() >= expected.len()
            && path
                .iter()
                .zip(expected.iter())
                .all(|(&actual, &expected)| {
                    let actual = if actual == b'/' {
                        b'\\'
                    } else {
                        actual.to_ascii_lowercase()
                    };
                    actual == expected
                })
            && (path.len() == expected.len()
                || path[expected.len()..].eq_ignore_ascii_case(b".hdp"))
    })
}
