//! One runtime thread owns input. Focus is checked immediately before each action.
use crate::engine::Action;
use anyhow::{anyhow, Result};
use enigo::{Button, Direction, Enigo, Keyboard, Mouse, Settings};

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
            Action::SelectSlot(slot) => select_slot(&mut self.0, *slot)?,
        }
        Ok(())
    }
}

/// Hotbar bindings are physical number-row keys on macOS, not text entry.
/// Enigo's Unicode conversion calls TISGetInputSourceProperty, which traps off
/// the main dispatch queue on current macOS. Keep that path out of the worker.
fn select_slot(keyboard: &mut impl Keyboard, slot: u8) -> Result<()> {
    if slot > 9 {
        return Err(anyhow!("Hotbar slot must be between 0 and 9"));
    }
    #[cfg(target_os = "macos")]
    {
        // Carbon Events.h: kVK_ANSI_0 through kVK_ANSI_9 (not sequential).
        const NUMBER_ROW: [u16; 10] = [0x1d, 0x12, 0x13, 0x14, 0x15, 0x17, 0x16, 0x1a, 0x1c, 0x19];
        keyboard.raw(NUMBER_ROW[usize::from(slot)], Direction::Click)?;
    }
    #[cfg(not(target_os = "macos"))]
    keyboard.key(
        enigo::Key::Unicode(char::from(b'0' + slot)),
        Direction::Click,
    )?;
    Ok(())
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use enigo::{InputResult, Key};

    #[derive(Default)]
    struct KeyboardProbe(Vec<(u16, Direction)>);
    impl Keyboard for KeyboardProbe {
        fn fast_text(&mut self, _: &str) -> InputResult<Option<()>> {
            panic!("Hotbar selection must not use text input");
        }
        fn key(&mut self, _: Key, _: Direction) -> InputResult<()> {
            panic!("Unicode key conversion can abort the macOS worker");
        }
        fn raw(&mut self, code: u16, direction: Direction) -> InputResult<()> {
            self.0.push((code, direction));
            Ok(())
        }
    }

    #[test]
    fn mac_hotbar_uses_raw_keys_on_a_worker_without_unicode_lookup() {
        std::thread::spawn(|| {
            let mut keyboard = KeyboardProbe::default();
            select_slot(&mut keyboard, 4).unwrap();
            select_slot(&mut keyboard, 5).unwrap();
            select_slot(&mut keyboard, 0).unwrap();
            assert_eq!(
                keyboard.0,
                vec![
                    (0x15, Direction::Click),
                    (0x17, Direction::Click),
                    (0x1d, Direction::Click)
                ]
            );
        })
        .join()
        .unwrap();
    }

    #[test]
    fn invalid_hotbar_slot_sends_no_input() {
        let mut keyboard = KeyboardProbe::default();
        assert!(select_slot(&mut keyboard, 10).is_err());
        assert!(select_slot(&mut keyboard, u8::MAX).is_err());
        assert!(keyboard.0.is_empty());
    }
}
