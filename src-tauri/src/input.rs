//! One runtime thread owns input. Focus is checked immediately before each action.
use crate::engine::Action;
use anyhow::{anyhow, Result};
use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse, Settings};

pub struct NativeInput(Enigo);
impl NativeInput {
    pub fn new() -> Result<Self> {
        Ok(Self(Enigo::new(&Settings::default())?))
    }
    pub fn execute(&mut self, action: &Action) -> Result<()> {
        if !roblox_focused()? {
            return Err(anyhow!("Roblox is no longer the foreground application"));
        }
        match action {
            Action::Click => self.0.button(Button::Left, Direction::Click)?,
            Action::SelectSlot(slot) => self
                .0
                .key(Key::Unicode(char::from(b'0' + slot)), Direction::Click)?,
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
pub fn roblox_focused() -> Result<bool> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::{c_char, CStr};
    // All objects are scoped to the worker's autorelease pool. No pointers escape.
    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: *mut Object = msg_send![workspace, frontmostApplication];
        let identifier: *mut Object = if app.is_null() {
            std::ptr::null_mut()
        } else {
            msg_send![app, bundleIdentifier]
        };
        let value: *const c_char = if identifier.is_null() {
            std::ptr::null()
        } else {
            msg_send![identifier, UTF8String]
        };
        let focused =
            !value.is_null() && CStr::from_ptr(value).to_bytes() == b"com.roblox.RobloxPlayer";
        let _: () = msg_send![pool, drain];
        Ok(focused)
    }
}

#[cfg(windows)]
pub fn roblox_focused() -> Result<bool> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };
    unsafe {
        let window = GetForegroundWindow();
        if window == 0 {
            return Ok(false);
        }
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process == 0 {
            return Ok(false);
        }
        let mut buffer = vec![0u16; 32768];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if ok == 0 {
            return Ok(false);
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        Ok(path
            .rsplit(['\\', '/'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("RobloxPlayerBeta.exe")))
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn roblox_focused() -> Result<bool> {
    Err(anyhow!(
        "Foreground verification is implemented for macOS and Windows only"
    ))
}
