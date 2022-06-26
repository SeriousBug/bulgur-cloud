use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::{io, iter};
use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

fn to_utf16(s: &Path) -> Vec<u16> {
    s.as_os_str().encode_wide().chain(iter::once(0)).collect()
}

pub fn rename<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> std::io::Result<()> {
    let from = to_utf16(from.as_ref());
    let to = to_utf16(to.as_ref());

    unsafe {
        if MoveFileExW(from.as_ptr(), to.as_ptr(), 0) == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
