//! Hand-written Win32 API bindings.
//!
//! Stash needs a handful of `user32`/`kernel32`/`ntdll` entry points that no
//! crate in the dependency tree exposes:
//!
//!  * `EnumWindows` + `GetWindowThreadProcessId` — map a process ID to the set of
//!    top-level windows it owns, so multiple instances of the same executable can
//!    be told apart (two VS Code windows, three Chrome windows, …).
//!  * `PostMessageW(WM_CLOSE)` — ask a window to close *gracefully* so the app can
//!    prompt to save, instead of `taskkill /F` which discards unsaved work.
//!  * `SetWindowPos` / `ShowWindow` — restore geometry, un-minimise first.
//!  * `NtQueryInformationProcess` + `ReadProcessMemory` — read another process's
//!    current working directory out of its PEB, which is how terminal windows get
//!    reopened in the directory they were actually working in.
//!
//! These are declared by hand rather than pulling in `windows-sys` to keep the
//! dependency tree (and the release binary) small — the crate already used this
//! approach for `SetWindowPos`.

#![cfg(target_os = "windows")]
#![allow(non_snake_case, non_camel_case_types)]

use std::os::windows::ffi::OsStringExt;

pub type Hwnd = isize;
pub type Handle = isize;
type Bool32 = i32;
type Dword = u32;
type Lparam = isize;
type Wparam = usize;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const WM_CLOSE: u32 = 0x0010;

const GW_OWNER: u32 = 4;
const GWL_EXSTYLE: i32 = -20;
const WS_EX_TOOLWINDOW: isize = 0x0000_0080;

const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_SHOWWINDOW: u32 = 0x0040;

const SW_RESTORE: i32 = 9;

const PROCESS_QUERY_INFORMATION: Dword = 0x0400;
const PROCESS_VM_READ: Dword = 0x0010;

const TH32CS_SNAPPROCESS: Dword = 0x0000_0002;
const MAX_PATH: usize = 260;

/// `DWM_CLOAKED` — set on the phantom UWP/"Windows Input Experience" windows that
/// are technically visible but never shown to the user.
const DWMWA_CLOAKED: Dword = 14;

// ── Externs ───────────────────────────────────────────────────────────────────

#[link(name = "user32")]
extern "system" {
    fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: Lparam) -> Bool32;
    fn GetWindowThreadProcessId(hWnd: Hwnd, lpdwProcessId: *mut Dword) -> Dword;
    fn IsWindowVisible(hWnd: Hwnd) -> Bool32;
    fn IsIconic(hWnd: Hwnd) -> Bool32;
    fn GetWindow(hWnd: Hwnd, uCmd: u32) -> Hwnd;
    fn GetWindowLongPtrW(hWnd: Hwnd, nIndex: i32) -> isize;
    fn GetWindowTextLengthW(hWnd: Hwnd) -> i32;
    fn PostMessageW(hWnd: Hwnd, Msg: u32, wParam: Wparam, lParam: Lparam) -> Bool32;
    fn ShowWindow(hWnd: Hwnd, nCmdShow: i32) -> Bool32;
    fn SetWindowPos(
        hWnd: Hwnd,
        hWndInsertAfter: Hwnd,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> Bool32;
}

#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(dwDesiredAccess: Dword, bInheritHandle: Bool32, dwProcessId: Dword) -> Handle;
    fn CloseHandle(hObject: Handle) -> Bool32;
    fn ReadProcessMemory(
        hProcess: Handle,
        lpBaseAddress: *const std::ffi::c_void,
        lpBuffer: *mut std::ffi::c_void,
        nSize: usize,
        lpNumberOfBytesRead: *mut usize,
    ) -> Bool32;
    fn CreateToolhelp32Snapshot(dwFlags: Dword, th32ProcessID: Dword) -> Handle;
    fn Process32FirstW(hSnapshot: Handle, lppe: *mut ProcessEntry32W) -> Bool32;
    fn Process32NextW(hSnapshot: Handle, lppe: *mut ProcessEntry32W) -> Bool32;
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQueryInformationProcess(
        ProcessHandle: Handle,
        ProcessInformationClass: u32,
        ProcessInformation: *mut std::ffi::c_void,
        ProcessInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> i32;
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmGetWindowAttribute(
        hwnd: Hwnd,
        dwAttribute: Dword,
        pvAttribute: *mut std::ffi::c_void,
        cbAttribute: Dword,
    ) -> i32;
}

type EnumWindowsProc = unsafe extern "system" fn(Hwnd, Lparam) -> Bool32;

#[repr(C)]
struct ProcessEntry32W {
    dwSize: Dword,
    cntUsage: Dword,
    th32ProcessID: Dword,
    th32DefaultHeapID: usize,
    th32ModuleID: Dword,
    cntThreads: Dword,
    th32ParentProcessID: Dword,
    pcPriClassBase: i32,
    dwFlags: Dword,
    szExeFile: [u16; MAX_PATH],
}

// ── Window enumeration ────────────────────────────────────────────────────────

struct EnumCtx {
    windows: Vec<(Hwnd, u32)>,
}

/// Callback invoked once per top-level window by `EnumWindows`.
unsafe extern "system" fn enum_proc(hwnd: Hwnd, lparam: Lparam) -> Bool32 {
    let ctx = unsafe { &mut *(lparam as *mut EnumCtx) };
    if is_user_window(hwnd) {
        let mut pid: Dword = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid != 0 {
            ctx.windows.push((hwnd, pid));
        }
    }
    1 // continue enumeration
}

/// True when the window is one a user would recognise: visible, not cloaked,
/// not a tool window, unowned, and carrying a non-empty title.
///
/// This is the same test the Alt-Tab list uses, and it is what keeps invisible
/// helper windows out of snapshots.
fn is_user_window(hwnd: Hwnd) -> bool {
    unsafe {
        if IsWindowVisible(hwnd) == 0 {
            return false;
        }
        // Owned windows are dialogs/popups belonging to another window.
        if GetWindow(hwnd, GW_OWNER) != 0 {
            return false;
        }
        if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return false;
        }
        // Cloaked windows (suspended UWP apps) report visible but are not drawn.
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut std::ffi::c_void,
            std::mem::size_of::<u32>() as Dword,
        );
        if hr == 0 && cloaked != 0 {
            return false;
        }
        true
    }
}

/// Every user-facing top-level window on the desktop, paired with its owning PID.
pub fn list_windows() -> Vec<(Hwnd, u32)> {
    let mut ctx = EnumCtx {
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(enum_proc, &mut ctx as *mut EnumCtx as Lparam);
    }
    ctx.windows
}

/// Top-level windows belonging to a specific process.
pub fn windows_for_pid(pid: u32) -> Vec<Hwnd> {
    list_windows()
        .into_iter()
        .filter(|(_, p)| *p == pid)
        .map(|(h, _)| h)
        .collect()
}

// ── Window actions ────────────────────────────────────────────────────────────

/// Move and resize a window, un-minimising it first so the geometry actually sticks.
pub fn set_position(hwnd: Hwnd, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        let ok = SetWindowPos(
            hwnd,
            0,
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        if ok == 0 {
            Err(format!("SetWindowPos failed for hwnd {hwnd}"))
        } else {
            Ok(())
        }
    }
}

/// Ask a window to close the same way clicking its ✕ button would.
///
/// The application receives `WM_CLOSE` and may show a "save your work?" prompt.
/// This is the graceful counterpart to `taskkill /F`.
pub fn close_window(hwnd: Hwnd) -> bool {
    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) != 0 }
}

/// Politely close every top-level window owned by `pid`.
/// Returns how many close requests were posted.
pub fn close_pid_windows(pid: u32) -> usize {
    let mut n = 0;
    for hwnd in windows_for_pid(pid) {
        if close_window(hwnd) {
            n += 1;
        }
    }
    n
}

// ── Process tree ──────────────────────────────────────────────────────────────

/// `(pid, parent_pid, exe_file_name)` for every process on the system.
pub fn process_list() -> Vec<(u32, u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == -1 || snap == 0 {
            return out;
        }
        let mut entry: ProcessEntry32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<ProcessEntry32W>() as Dword;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = std::ffi::OsString::from_wide(&entry.szExeFile[..end])
                    .to_string_lossy()
                    .into_owned();
                out.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    out
}

// ── Current working directory of another process ──────────────────────────────

/// Read a process's current working directory out of its PEB.
///
/// Walks `PROCESS_BASIC_INFORMATION -> PEB -> RTL_USER_PROCESS_PARAMETERS ->
/// CurrentDirectory.DosPath`. Only implemented for 64-bit builds; the structure
/// offsets differ under WOW64 and the payoff is not worth the branch.
#[cfg(target_pointer_width = "64")]
pub fn process_cwd(pid: u32) -> Option<String> {
    process_cwd_detailed(pid).ok()
}

/// Same as [`process_cwd`] but naming the step that failed, so a terminal that
/// will not give up its directory can be diagnosed instead of silently ignored.
#[cfg(target_pointer_width = "64")]
pub fn process_cwd_detailed(pid: u32) -> Result<String, String> {
    // Offsets for the 64-bit layout of the undocumented process structures.
    const PEB_OFFSET_IN_PBI: usize = 0x08; // PROCESS_BASIC_INFORMATION.PebBaseAddress
    const PARAMS_OFFSET_IN_PEB: usize = 0x20; // PEB.ProcessParameters
    const CURDIR_OFFSET_IN_PARAMS: usize = 0x38; // RTL_USER_PROCESS_PARAMETERS.CurrentDirectory
    const UNICODE_STRING_BUFFER_OFFSET: usize = 0x08; // UNICODE_STRING.Buffer

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle == 0 {
            // Higher-integrity or protected process.
            return Err(format!("OpenProcess({pid}) denied"));
        }
        let guard = HandleGuard(handle);

        // 1. PROCESS_BASIC_INFORMATION (48 bytes on x64)
        let mut pbi = [0u8; 48];
        let status = NtQueryInformationProcess(
            guard.0,
            0, // ProcessBasicInformation
            pbi.as_mut_ptr() as *mut std::ffi::c_void,
            pbi.len() as u32,
            std::ptr::null_mut(),
        );
        if status != 0 {
            return Err(format!("NtQueryInformationProcess status {status:#x}"));
        }
        let peb_addr = usize::from_ne_bytes(
            pbi[PEB_OFFSET_IN_PBI..PEB_OFFSET_IN_PBI + 8]
                .try_into()
                .map_err(|_| "PBI too short".to_string())?,
        );
        if peb_addr == 0 {
            return Err("PEB address is null".into());
        }

        // 2. PEB.ProcessParameters
        let params_addr: usize = read_pod(guard.0, peb_addr + PARAMS_OFFSET_IN_PEB)
            .ok_or_else(|| format!("could not read PEB at {peb_addr:#x}"))?;
        if params_addr == 0 {
            return Err("ProcessParameters is null".into());
        }

        // 3. CurrentDirectory.DosPath — a UNICODE_STRING { u16 len, u16 max, *u16 buf }
        let curdir = params_addr + CURDIR_OFFSET_IN_PARAMS;
        let len_bytes: u16 = read_pod(guard.0, curdir)
            .ok_or_else(|| format!("could not read CurrentDirectory at {curdir:#x}"))?;
        if len_bytes == 0 {
            return Err("CurrentDirectory is empty".into());
        }
        if len_bytes as usize > MAX_CWD_BYTES {
            return Err(format!("CurrentDirectory length {len_bytes} is implausible"));
        }
        let buf_addr: usize = read_pod(guard.0, curdir + UNICODE_STRING_BUFFER_OFFSET)
            .ok_or_else(|| "could not read CurrentDirectory.Buffer".to_string())?;
        if buf_addr == 0 {
            return Err("CurrentDirectory.Buffer is null".into());
        }

        // 4. The path itself, as UTF-16.
        let mut raw = vec![0u8; len_bytes as usize];
        let mut read = 0usize;
        let ok = ReadProcessMemory(
            guard.0,
            buf_addr as *const std::ffi::c_void,
            raw.as_mut_ptr() as *mut std::ffi::c_void,
            raw.len(),
            &mut read,
        );
        if ok == 0 {
            return Err(format!("ReadProcessMemory failed at {buf_addr:#x}"));
        }
        if read != raw.len() {
            return Err(format!("short read: {read} of {} bytes", raw.len()));
        }
        let utf16: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_ne_bytes([c[0], c[1]]))
            .collect();
        let path = String::from_utf16_lossy(&utf16);
        let trimmed = path.trim_end_matches(['\\', '\0']).to_string();
        if trimmed.is_empty() {
            Err("CurrentDirectory decoded to an empty string".into())
        } else {
            Ok(trimmed)
        }
    }
}

#[cfg(not(target_pointer_width = "64"))]
pub fn process_cwd(_pid: u32) -> Option<String> {
    None
}

const MAX_CWD_BYTES: usize = 4096;

/// Read a `Copy` value out of another process's address space.
#[cfg(target_pointer_width = "64")]
unsafe fn read_pod<T: Copy>(handle: Handle, addr: usize) -> Option<T> {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    let mut read = 0usize;
    let ok = unsafe {
        ReadProcessMemory(
            handle,
            addr as *const std::ffi::c_void,
            value.as_mut_ptr() as *mut std::ffi::c_void,
            std::mem::size_of::<T>(),
            &mut read,
        )
    };
    if ok == 0 || read != std::mem::size_of::<T>() {
        return None;
    }
    Some(unsafe { value.assume_init() })
}

/// Closes the wrapped handle on drop so every early `return None` above is safe.
struct HandleGuard(Handle);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { CloseHandle(self.0) };
        }
    }
}
