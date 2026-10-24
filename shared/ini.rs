//! 两个插件共用的无分配 INI 输出，保留模板中的逐项说明。
// 两个 NRO 使用不同的数值类型与文本接口；未用到的部分由链接器剔除。
#![allow(dead_code)]
use core::{
    ffi::CStr,
    fmt::{self, Write},
};

pub struct Text<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Text<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
    pub fn as_c_str(&mut self) -> &CStr {
        self.bytes[self.len] = 0;
        CStr::from_bytes_with_nul(&self.bytes[..=self.len]).unwrap()
    }
}

impl<const N: usize> Write for Text<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() >= N - self.len {
            return Err(fmt::Error);
        }
        self.bytes[self.len..self.len + text.len()].copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}

pub enum Value {
    Bool(bool),
    Integer(u32),
    Float(f32),
    IdSet(u8),
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => v.fmt(f),
            Self::Integer(v) => v.fmt(f),
            Self::Float(v) => v.fmt(f),
            Self::IdSet(bits) => {
                if *bits == 0 {
                    return f.write_str("none");
                }
                if *bits == u8::MAX {
                    return f.write_str("all");
                }
                let mut comma = false;
                for id in 0..8 {
                    if bits & (1 << id) == 0 {
                        continue;
                    }
                    if comma {
                        f.write_str(",")?;
                    }
                    write!(f, "{id}")?;
                    comma = true;
                }
                Ok(())
            }
        }
    }
}

pub fn rewrite(
    template: &str,
    out: &mut dyn Write,
    mut value: impl FnMut(&str, &str) -> Option<Value>,
) -> fmt::Result {
    let mut section = "";
    for line in template.lines() {
        let body = line.split([';', '#']).next().unwrap().trim();
        if let Some(name) = body.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name;
        }
        if let Some((key, _)) = body.split_once('=') {
            if let Some(v) = value(section, key.trim()) {
                write!(out, "{} = {v}", key.trim())?;
                if let Some(at) = line.find([';', '#']) {
                    write!(out, " {}", &line[at..])?;
                }
                writeln!(out)?;
                continue;
            }
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}
