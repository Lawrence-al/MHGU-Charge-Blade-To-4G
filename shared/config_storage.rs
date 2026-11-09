//! 自动选择原版存档文件系统或重编译的plugins宿主文件桥。
#![allow(dead_code)]

use core::ffi::{CStr, c_char, c_void};
use forge::log::prelude::{info, warn};
use forge::os::sync::OnceLock;

pub const MAX_BYTES: usize = 8192;

pub struct Location {
    pub save_path: &'static CStr,
    pub plugin_path: &'static CStr,
    pub defaults: &'static str,
}

macro_rules! location {
    ($name:ident, $file:literal, $defaults:literal) => {
        pub const $name: Location = Location {
            save_path: unsafe {
                CStr::from_bytes_with_nul_unchecked(concat!("savedata:/", $file, "\0").as_bytes())
            },
            plugin_path: unsafe {
                CStr::from_bytes_with_nul_unchecked(concat!("app:/nativeNX/plugins/", $file, "\0").as_bytes())
            },
            defaults: include_str!($defaults),
        };
    };
}

location!(
    PRESENTATION,
    "cb_presentation_to_4g.ini",
    "../conf/cb_presentation_to_4g.ini"
);
location!(
    MOVESET,
    "cb_moveset_to_4g.ini",
    "../conf/cb_moveset_to_4g.ini"
);
location!(STYLE, "cb_style_to_4g.ini", "../conf/cb_style_to_4g.ini");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    Environment,
    Unavailable,
    Io,
    TooLarge,
    Utf8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FileHandle(u32);
#[repr(C)]
struct WriteOption(i32);
type ImFile = *mut c_void;
const _: () = assert!(core::mem::size_of::<FileHandle>() == 4);

unsafe extern "C" {
    fn forge_fs_mountSaveData() -> bool;
    fn forge_fs_unmountSaveData();
    #[link_name = "_ZN2nn2fs14CommitSaveDataEPKc"]
    fn commit_save(name: *const c_char) -> u32;
    #[link_name = "igImFileOpen"]
    fn im_open(path: *const c_char, mode: *const c_char) -> ImFile;
    #[link_name = "igImFileClose"]
    fn im_close(file: ImFile) -> bool;
    #[link_name = "igImFileGetSize"]
    fn im_size(file: ImFile) -> u64;
    #[link_name = "igImFileRead"]
    fn im_read(bytes: *mut c_void, size: u64, count: u64, file: ImFile) -> u64;
    #[link_name = "igImFileWrite"]
    fn im_write(bytes: *const c_void, size: u64, count: u64, file: ImFile) -> u64;
    #[link_name = "_ZN2nn2fs8OpenFileEPNS0_10FileHandleEPKci"]
    fn open_file(handle: *mut FileHandle, path: *const c_char, mode: i32) -> u32;
    #[link_name = "_ZN2nn2fs11GetFileSizeEPxNS0_10FileHandleE"]
    fn get_file_size(size: *mut i64, handle: FileHandle) -> u32;
    #[link_name = "_ZN2nn2fs8ReadFileENS0_10FileHandleExPvj"]
    fn read_file(handle: FileHandle, offset: i64, bytes: *mut c_void, size: usize) -> u32;
    #[link_name = "_ZN2nn2fs10CreateFileEPKcx"]
    fn create_file(path: *const c_char, size: i64) -> u32;
    #[link_name = "_ZN2nn2fs11SetFileSizeENS0_10FileHandleEx"]
    fn set_file_size(handle: FileHandle, size: i64) -> u32;
    #[link_name = "_ZN2nn2fs9WriteFileENS0_10FileHandleExPKvjRKNS0_11WriteOptionE"]
    fn write_file(
        handle: FileHandle,
        offset: i64,
        bytes: *const c_void,
        size: usize,
        option: *const WriteOption,
    ) -> u32;
    #[link_name = "_ZN2nn2fs9CloseFileENS0_10FileHandleE"]
    fn close_file(handle: FileHandle);
}

#[derive(Clone, Copy)]
enum Backend {
    SaveData,
    Plugins,
}

static BACKEND: OnceLock<Result<Backend, ReadError>> = OnceLock::new();

fn backend() -> Result<Backend, ReadError> {
    *BACKEND.get_or_init(|| {
        // 此资源是GU本体必备文件。只打开读取，不读取ARC内容或修改资源。
        let file = unsafe {
            im_open(c"app:/nativeNX/arc/player/quest/w14.arc".as_ptr(), c"rb".as_ptr())
        };
        if file.is_null() {
            warn!("Could not open the game resource used to identify config storage");
            return Err(ReadError::Environment);
        }
        // 三套重编译把宿主FILE映射成1..32号句柄。原版返回位于有效内存中的FILE指针。
        // 使用保留的首个内存页区分句柄和指针，兼容句柄表增大，不依赖模块基址。
        let selected = if (file as usize) < 0x1000 {
            Backend::Plugins
        } else {
            Backend::SaveData
        };
        if !unsafe { im_close(file) } {
            return Err(ReadError::Environment);
        }
        Ok(selected)
    })
}

struct SaveMount;
impl SaveMount {
    fn open() -> Result<Self, ReadError> {
        if unsafe { forge_fs_mountSaveData() } {
            Ok(Self)
        } else {
            warn!("Could not mount MHGU savedata");
            Err(ReadError::Unavailable)
        }
    }
}
impl Drop for SaveMount {
    fn drop(&mut self) {
        unsafe { forge_fs_unmountSaveData() };
    }
}

fn missing(code: u32) -> bool {
    code & 0x3F_FFFF == 0x202
}

impl Location {
    pub fn path(&self) -> Result<&'static CStr, ReadError> {
        Ok(match backend()? {
            Backend::SaveData => self.save_path,
            Backend::Plugins => self.plugin_path,
        })
    }

    pub fn display(&self) -> &'static str {
        match self.path() {
            Ok(path) => path.to_str().unwrap(),
            Err(_) => "Configuration storage could not be identified",
        }
    }

    pub fn read<'a>(&self, bytes: &'a mut [u8]) -> Result<&'a str, ReadError> {
        match backend()? {
            Backend::SaveData => self.read_native(bytes),
            Backend::Plugins => self.read_plugins(bytes),
        }
    }

    fn read_native<'a>(&self, bytes: &'a mut [u8]) -> Result<&'a str, ReadError> {
        let _mount = SaveMount::open()?;
        let mut file = FileHandle(0);
        let code = unsafe { open_file(&mut file, self.save_path.as_ptr(), 1) };
        if missing(code) {
            if self.defaults.len() > bytes.len() {
                return Err(ReadError::TooLarge);
            }
            self.write_native(self.defaults)
                .map_err(|_| ReadError::Io)?;
            bytes[..self.defaults.len()].copy_from_slice(self.defaults.as_bytes());
            info!("{}: generated default INI", self.display());
            return Ok(core::str::from_utf8(&bytes[..self.defaults.len()]).unwrap());
        }
        if code != 0 {
            warn!("{}: open failed 0x{code:08X}", self.display());
            return Err(ReadError::Io);
        }
        let mut size = 0i64;
        let mut code = unsafe { get_file_size(&mut size, file) };
        if code == 0 && (0..=bytes.len() as i64).contains(&size) {
            code = unsafe { read_file(file, 0, bytes.as_mut_ptr().cast(), size as usize) };
        }
        unsafe { close_file(file) };
        if code != 0 {
            warn!("{}: read failed 0x{code:08X}", self.display());
            return Err(ReadError::Io);
        }
        if !(0..=bytes.len() as i64).contains(&size) {
            return Err(ReadError::TooLarge);
        }
        let text = core::str::from_utf8(&bytes[..size as usize]).map_err(|_| ReadError::Utf8)?;
        info!("{}: loaded", self.display());
        Ok(text)
    }

    pub fn load<C: Copy, E: core::fmt::Debug>(
        &self,
        default: C,
        parse: impl FnOnce(&str) -> Result<C, E>,
    ) -> C {
        let mut bytes = [0; MAX_BYTES];
        match self.read(&mut bytes) {
            Ok(text) => match parse(text) {
                Ok(config) => return config,
                Err(problem) => warn!("{}: {problem:?}; using defaults", self.display()),
            },
            Err(problem) => warn!("{}: {problem:?}; using defaults", self.display()),
        }
        default
    }

    pub fn save(&self, text: &str) -> Result<(), &'static str> {
        if text.len() > MAX_BYTES {
            return Err("Configuration is too large.");
        }
        match backend().map_err(|_| "Could not identify configuration storage. See Forge log.")? {
            Backend::SaveData => {
                let _mount = SaveMount::open()
                    .map_err(|_| "Could not mount MHGU save data. See Forge log.")?;
                self.write_native(text)
            }
            Backend::Plugins => self.write_plugins(text),
        }
    }

    pub fn save_text(&self, text: &str) -> Result<(), &'static str> {
        self.save(text)
    }

    fn write_native(&self, text: &str) -> Result<(), &'static str> {
        let mut file = FileHandle(0);
        let code = unsafe { open_file(&mut file, self.save_path.as_ptr(), 6) };
        if missing(code) {
            let code = unsafe { create_file(self.save_path.as_ptr(), 0) };
            if code != 0 {
                return self.failed(code, "Could not create plugin INI. See Forge log.");
            }
            let code = unsafe { open_file(&mut file, self.save_path.as_ptr(), 6) };
            if code != 0 {
                return self.failed(code, "Could not open plugin INI for writing.");
            }
        } else if code != 0 {
            return self.failed(code, "Could not open plugin INI for writing.");
        }
        let mut code = unsafe { set_file_size(file, text.len() as i64) };
        if code == 0 {
            code =
                unsafe { write_file(file, 0, text.as_ptr().cast(), text.len(), &WriteOption(1)) };
        }
        unsafe { close_file(file) };
        if code != 0 {
            return self.failed(code, "Could not write plugin INI. See Forge log.");
        }
        let code = unsafe { commit_save(c"savedata".as_ptr()) };
        if code != 0 {
            return self.failed(code, "Could not commit MHGU save INI. See Forge log.");
        }
        info!("{}: saved", self.display());
        Ok(())
    }

    fn read_plugins<'a>(&self, bytes: &'a mut [u8]) -> Result<&'a str, ReadError> {
        let file = unsafe { im_open(self.plugin_path.as_ptr(), c"rb".as_ptr()) };
        if file.is_null() {
            // 重编译的app路径映射需要启动时已有该资源，因此默认INI随NRO发布。
            warn!("{}: could not open INI; install its default file beside the NRO", self.display());
            if self.defaults.len() > bytes.len() {
                return Err(ReadError::TooLarge);
            }
            bytes[..self.defaults.len()].copy_from_slice(self.defaults.as_bytes());
            return Ok(core::str::from_utf8(&bytes[..self.defaults.len()]).unwrap());
        }
        let size = unsafe { im_size(file) };
        let fits = size <= bytes.len() as u64;
        let read = fits && unsafe { im_read(bytes.as_mut_ptr().cast(), 1, size, file) == size };
        let closed = unsafe { im_close(file) };
        if size == u64::MAX || !closed {
            return Err(ReadError::Io);
        }
        if !fits {
            return Err(ReadError::TooLarge);
        }
        if !read {
            return Err(ReadError::Io);
        }
        let text = core::str::from_utf8(&bytes[..size as usize]).map_err(|_| ReadError::Utf8)?;
        info!("{}: loaded", self.display());
        Ok(text)
    }

    fn write_plugins(&self, text: &str) -> Result<(), &'static str> {
        let file = unsafe { im_open(self.plugin_path.as_ptr(), c"wb".as_ptr()) };
        if file.is_null() {
            warn!("{}: host file open failed", self.display());
            return Err("Could not open plugins INI for writing. Install its default file beside the NRO.");
        }
        let written = unsafe { im_write(text.as_ptr().cast(), 1, text.len() as u64, file) };
        let closed = unsafe { im_close(file) };
        if written != text.len() as u64 || !closed {
            warn!("{}: host write returned {written}/{} bytes, close={closed}", self.display(), text.len());
            return Err("Could not write plugins INI. See Forge log.");
        }
        info!("{}: saved", self.display());
        Ok(())
    }

    fn failed(&self, code: u32, message: &'static str) -> Result<(), &'static str> {
        warn!("{}: save failed 0x{code:08X}", self.display());
        Err(message)
    }
}
