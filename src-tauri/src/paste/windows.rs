use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::ptr::copy_nonoverlapping;
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{GetLastError, GlobalFree, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

const VK_V: u16 = b'V' as u16;

pub fn active_window_id() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        None
    } else {
        Some((hwnd as usize).to_string())
    }
}

struct ClipboardGuard {
    open: bool,
}

impl ClipboardGuard {
    fn open(owner: HWND) -> Result<Self> {
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            let ok = unsafe { OpenClipboard(owner) != 0 };
            if ok {
                return Ok(Self { open: true });
            }
            if Instant::now() >= deadline {
                return Err(anyhow!("OpenClipboard failed: {}", unsafe {
                    GetLastError()
                }));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn owner_window(target_window: Option<&str>) -> HWND {
    target_window
        .and_then(|id| id.parse::<usize>().ok())
        .map(|id| id as HWND)
        .filter(|hwnd| !hwnd.is_null())
        .unwrap_or_else(|| unsafe { GetForegroundWindow() })
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        if self.open {
            unsafe {
                CloseClipboard();
            }
        }
    }
}

fn read_clipboard_text() -> Option<String> {
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT as u32) == 0 } {
        return None;
    }

    let handle = unsafe { GetClipboardData(CF_UNICODETEXT as u32) };
    if handle.is_null() {
        return None;
    }

    let ptr = unsafe { GlobalLock(handle) } as *const u16;
    if ptr.is_null() {
        return None;
    }

    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
    }

    let text = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf16_lossy(slice)
    };

    unsafe {
        GlobalUnlock(handle);
    }

    Some(text)
}

fn set_clipboard_text(text: &str) -> Result<()> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);

    let bytes = wide.len() * size_of::<u16>();
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
    if handle.is_null() {
        return Err(anyhow!("GlobalAlloc failed: {}", unsafe { GetLastError() }));
    }

    let ptr = unsafe { GlobalLock(handle) } as *mut u16;
    if ptr.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err(anyhow!("GlobalLock failed: {}", unsafe { GetLastError() }));
    }

    unsafe {
        copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        GlobalUnlock(handle);
    }

    if unsafe { SetClipboardData(CF_UNICODETEXT as u32, handle) }.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err(anyhow!("SetClipboardData failed: {}", unsafe {
            GetLastError()
        }));
    }

    Ok(())
}

fn replace_clipboard_text(text: &str) -> Result<()> {
    unsafe {
        if EmptyClipboard() == 0 {
            return Err(anyhow!("EmptyClipboard failed: {}", GetLastError()));
        }
    }
    set_clipboard_text(text)
}

fn key_input(vk: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_ctrl_v() -> Result<()> {
    let inputs = [
        key_input(VK_CONTROL as u16, false),
        key_input(VK_V, false),
        key_input(VK_V, true),
        key_input(VK_CONTROL as u16, true),
    ];

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(anyhow!(
            "SendInput Ctrl+V sent {sent}/{} events: {}",
            inputs.len(),
            unsafe { GetLastError() }
        ));
    }
    Ok(())
}

pub fn type_text(text: &str, target_window: Option<&str>) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    let owner = owner_window(target_window);
    let previous_text = {
        let _clipboard = ClipboardGuard::open(owner)?;
        let previous_text = read_clipboard_text();
        replace_clipboard_text(text)?;
        previous_text
    };

    thread::sleep(Duration::from_millis(30));
    send_ctrl_v()?;
    thread::sleep(Duration::from_millis(500));

    if let Some(previous_text) = previous_text {
        let _clipboard = ClipboardGuard::open(owner)?;
        replace_clipboard_text(&previous_text)?;
    }

    Ok(())
}
