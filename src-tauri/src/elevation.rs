//! Windows requires administrator rights to create the TUN adapter (via
//! Wintun) that sing-box uses. If Wisp isn't running elevated, relaunch
//! itself with a UAC prompt and let the (non-elevated) original process
//! exit.
//!
//! This module is `cfg(windows)`-gated end to end: on other platforms
//! [`ensure_elevated`] is a no-op that always reports "already elevated" so
//! callers don't need their own `cfg` branches.

#[cfg(windows)]
pub fn ensure_elevated() -> bool {
    if is_elevated() {
        return true;
    }
    relaunch_elevated();
    false
}

#[cfg(not(windows))]
pub fn ensure_elevated() -> bool {
    true
}

#[cfg(windows)]
fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned_len = 0u32;
        let queried = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut core::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned_len,
        )
        .is_ok();

        let _ = CloseHandle(token);
        queried && elevation.TokenIsElevated != 0
    }
}

#[cfg(windows)]
fn relaunch_elevated() {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Ok(exe) = std::env::current_exe() else {
        tracing::error!("elevation: could not determine current executable path");
        return;
    };
    let exe_path = HSTRING::from(exe.as_os_str());

    // SAFETY: all string args are valid, NUL-terminated wide strings for the
    // duration of this call (`w!` is a static literal, `exe_path` is an
    // owned `HSTRING` kept alive on the stack until `ShellExecuteW` returns).
    unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            &exe_path,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}
