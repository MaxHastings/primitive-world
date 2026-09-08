//! Small Windows integration layer for the wallpaper viewer.
//!
//! This deliberately uses per-user startup and desktop-window hosting rather
//! than a Windows service: services cannot own an interactive desktop window.

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::RefCell,
    ffi::c_void,
    ptr::null_mut,
    sync::{
        atomic::{AtomicIsize, AtomicU32, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, RECT},
    Graphics::Gdi::ScreenToClient,
    System::{
        Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
            RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
        },
        Threading::CreateMutexW,
    },
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
            Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
            DestroyMenu, DestroyWindow, EnumChildWindows, EnumWindows, FindWindowExW, FindWindowW,
            GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetClientRect, GetCursorPos, GetParent,
            GetWindowLongPtrW, GetWindowTextW, HC_ACTION, HHOOK, HWND_BOTTOM, HWND_MESSAGE,
            IDI_APPLICATION, IsWindow, IsWindowVisible, LWA_ALPHA, LoadIconW, MF_STRING,
            MSLLHOOKSTRUCT, PostMessageW, RegisterClassExW, SMTO_NORMAL, SW_HIDE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOSENDCHANGING, SWP_SHOWWINDOW, SendMessageTimeoutW,
            SetForegroundWindow, SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW,
            SetWindowPos, SetWindowsHookExW, ShowWindow, TPM_NONOTIFY, TPM_RETURNCMD,
            TPM_RIGHTBUTTON, TrackPopupMenu, UnhookWindowsHookEx, WH_MOUSE_LL, WM_CLOSE,
            WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_RBUTTONUP, WM_USER, WNDCLASSEXW, WS_CHILD,
            WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WindowFromPoint,
        },
    },
};
use winit::window::Window;

const STARTUP_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_VALUE: &str = "PrimitiveWorld";
const MUTEX_NAME: &str = "Local\\PrimitiveWorld.Wallpaper.Singleton";
const DESKTOP_MESSAGE: u32 = 0x052C;
const DESKTOP_MESSAGE_WPARAM: usize = 0xD;
const DESKTOP_MESSAGE_LPARAM: LPARAM = 1;
const TRAY_CALLBACK: u32 = WM_USER + 1;
const TRAY_PAUSE: u32 = 1;
const TRAY_QUIT: u32 = 2;
static TRAY_ACTION: AtomicU32 = AtomicU32::new(0);
static WALLPAPER_WINDOW: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone, Copy)]
struct DesktopClick {
    x: i32,
    y: i32,
    time: Instant,
}

thread_local! {
    // WH_MOUSE_LL runs on the installing thread. Never wait for Explorer here.
    static CLICK_SENDER: RefCell<Option<SyncSender<DesktopClick>>> = const { RefCell::new(None) };
}

pub struct SingleInstance {
    handle: HANDLE,
}

pub struct Tray {
    window: HWND,
    icon: NOTIFYICONDATAW,
    mouse_hook: HHOOK,
    clicks: Receiver<DesktopClick>,
}

impl Tray {
    pub fn new() -> Result<Self, String> {
        let class = wide("PrimitiveWorldTrayWindow");
        let window_name = wide("Primitive World");
        // SAFETY: the class structure contains valid pointers for the duration
        // of registration and uses a process-local callback.
        unsafe {
            let class_info = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(tray_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: null_mut(),
                hIcon: null_mut(),
                hCursor: null_mut(),
                hbrBackground: null_mut(),
                lpszMenuName: null_mut(),
                lpszClassName: class.as_ptr(),
                hIconSm: null_mut(),
            };
            RegisterClassExW(&class_info);
        }
        // SAFETY: the class name and window title are valid strings. This is a
        // hidden message-only window owned by the current event-loop thread.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                window_name.as_ptr(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        if window.is_null() {
            return Err("Could not create the wallpaper tray window".into());
        }
        // SAFETY: the icon handle is a system-owned stock icon; the tray data
        // is initialized before Shell_NotifyIconW reads it.
        let mut icon: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        icon.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        icon.hWnd = window;
        icon.uID = 1;
        icon.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        icon.uCallbackMessage = TRAY_CALLBACK;
        icon.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
        let tip = wide("Primitive World wallpaper");
        let tip_len = tip.len().min(icon.szTip.len());
        icon.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);
        // SAFETY: window and icon data are valid for the tray registration.
        if unsafe { Shell_NotifyIconW(NIM_ADD, &icon) } == 0 {
            // SAFETY: window was created above and is owned by this object.
            unsafe { DestroyWindow(window) };
            return Err("Windows rejected the wallpaper tray icon".into());
        }
        let (sender, clicks) = match start_click_worker() {
            Ok(worker) => worker,
            Err(error) => {
                unsafe {
                    Shell_NotifyIconW(NIM_DELETE, &icon);
                    DestroyWindow(window);
                }
                return Err(error);
            }
        };
        CLICK_SENDER.with(|slot| *slot.borrow_mut() = Some(sender));
        // Observe without consuming input. Cross-process accessibility work is
        // performed by the worker, never from this low-level mouse hook.
        let mouse_hook =
            unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(desktop_mouse_hook), null_mut(), 0) };
        if mouse_hook.is_null() {
            CLICK_SENDER.with(|slot| slot.borrow_mut().take());
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &icon);
                DestroyWindow(window);
            }
            return Err(format!("Could not observe desktop clicks ({})", unsafe {
                GetLastError()
            }));
        }
        // SAFETY: this only hides the helper window; the tray icon remains.
        unsafe { ShowWindow(window, SW_HIDE) };
        Ok(Self {
            window,
            icon,
            mouse_hook,
            clicks,
        })
    }

    pub fn take_action(&self) -> Option<TrayAction> {
        match TRAY_ACTION.swap(0, Ordering::AcqRel) {
            x if x == TRAY_PAUSE => Some(TrayAction::TogglePause),
            x if x == TRAY_QUIT => Some(TrayAction::Quit),
            _ => self
                .clicks
                .try_iter()
                .find(|click| click.time.elapsed() < Duration::from_millis(500))
                .and_then(|click| {
                    let wallpaper = WALLPAPER_WINDOW.load(Ordering::Acquire) as HWND;
                    let mut point = windows_sys::Win32::Foundation::POINT {
                        x: click.x,
                        y: click.y,
                    };
                    if unsafe { IsWindow(wallpaper) } != 0
                        && unsafe { ScreenToClient(wallpaper, &mut point) } != 0
                    {
                        Some(TrayAction::DesktopClick {
                            x: point.x,
                            y: point.y,
                        })
                    } else {
                        None
                    }
                }),
        }
    }

    pub fn refresh(&self) {
        // Explorer discards notification icons on restart. Probe the existing
        // registration before adding it again, avoiding duplicate icons.
        unsafe {
            if Shell_NotifyIconW(NIM_MODIFY, &self.icon) == 0 {
                Shell_NotifyIconW(NIM_ADD, &self.icon);
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        CLICK_SENDER.with(|slot| slot.borrow_mut().take());
        WALLPAPER_WINDOW.store(0, Ordering::Release);
        // SAFETY: the icon and helper window belong to this object.
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.icon);
            UnhookWindowsHookEx(self.mouse_hook);
            DestroyWindow(self.window);
        }
    }
}

pub enum TrayAction {
    TogglePause,
    Quit,
    DesktopClick { x: i32, y: i32 },
}

unsafe extern "system" fn desktop_mouse_hook(code: i32, wparam: usize, lparam: LPARAM) -> isize {
    if code == HC_ACTION as i32 && wparam as u32 == WM_LBUTTONDOWN {
        // SAFETY: Windows supplies this structure for HC_ACTION.
        let screen = unsafe { (*(lparam as *const MSLLHOOKSTRUCT)).pt };
        CLICK_SENDER.with(|slot| {
            if let Ok(sender) = slot.try_borrow()
                && let Some(sender) = sender.as_ref()
            {
                let _ = sender.try_send(DesktopClick {
                    x: screen.x,
                    y: screen.y,
                    time: Instant::now(),
                });
            }
        });
    }
    // Observational only: Explorer and all other applications still receive the click.
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

fn desktop_background_was_clicked(screen: windows_sys::Win32::Foundation::POINT) -> bool {
    let target = unsafe { WindowFromPoint(screen) };
    if target.is_null() || window_class(target) != "SysListView32" {
        return false;
    }
    // Only Explorer's desktop list, never list views belonging to other apps.
    if window_class(unsafe { GetParent(target) }) != "SHELLDLL_DefView" {
        return false;
    }
    use windows::{
        Win32::UI::Accessibility::{AccessibleObjectFromPoint, ROLE_SYSTEM_LIST},
        core::VARIANT,
    };
    let mut accessible = None;
    let mut child = VARIANT::default();
    // MSAA marshals cross-process data. Do NOT use LVM_HITTEST with a local
    // pointer: Explorer would dereference memory from the wrong process.
    if unsafe {
        AccessibleObjectFromPoint(
            windows::Win32::Foundation::POINT {
                x: screen.x,
                y: screen.y,
            },
            &mut accessible,
            &mut child,
        )
    }
    .is_err()
        || i32::try_from(&child) != Ok(0)
    {
        return false;
    }
    let Some(accessible) = accessible else {
        return false;
    };
    let role = unsafe { accessible.get_accRole(&child) };
    role.ok().and_then(|role| i32::try_from(&role).ok()) == Some(ROLE_SYSTEM_LIST as i32)
        && unsafe { WindowFromPoint(screen) } == target
}

fn start_click_worker() -> Result<(SyncSender<DesktopClick>, Receiver<DesktopClick>), String> {
    let (sender, requests) = mpsc::sync_channel::<DesktopClick>(16);
    let (results, receiver) = mpsc::sync_channel(16);
    std::thread::Builder::new()
        .name("desktop-hit-test".into())
        .spawn(move || {
            use windows::Win32::System::Com::{
                COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
            };
            // COM objects stay on this worker and are dropped before uninitializing.
            if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_err() {
                eprintln!("Desktop accessibility initialization failed; desktop clicks disabled");
                return;
            }
            while let Ok(click) = requests.recv() {
                if click.time.elapsed() < Duration::from_millis(500)
                    && desktop_background_was_clicked(windows_sys::Win32::Foundation::POINT {
                        x: click.x,
                        y: click.y,
                    })
                {
                    let _ = results.try_send(click);
                }
            }
            unsafe { CoUninitialize() };
        })
        .map_err(|error| format!("Could not start desktop hit testing: {error}"))?;
    Ok((sender, receiver))
}

fn window_class(window: HWND) -> String {
    let mut class = [0u16; 256];
    let len = unsafe { GetClassNameW(window, class.as_mut_ptr(), class.len() as i32) };
    String::from_utf16_lossy(&class[..len.max(0) as usize])
}

/// Ask the desktop child to take its normal save-before-close path.
pub fn stop_wallpaper() -> Result<String, String> {
    let mut windows: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(Some(find_wallpaper_host), &mut windows as *mut _ as LPARAM);
    }
    match windows.as_slice() {
        [] => Ok("No attached Primitive World wallpaper is running".into()),
        [window] => {
            if unsafe { PostMessageW(*window, WM_CLOSE, 0, 0) } == 0 {
                return Err("Could not request wallpaper shutdown".into());
            }
            Ok("Wallpaper asked to save and close; a save failure keeps it open".into())
        }
        _ => Err("Multiple wallpaper windows found; close them through their tray menus".into()),
    }
}

unsafe extern "system" fn find_wallpaper_host(hwnd: HWND, data: LPARAM) -> i32 {
    let mut class = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
    let class = String::from_utf16_lossy(&class[..len.max(0) as usize]);
    if class == "WorkerW" || class == "Progman" {
        unsafe {
            EnumChildWindows(hwnd, Some(find_wallpaper_child), data);
        }
    }
    1
}

unsafe extern "system" fn find_wallpaper_child(hwnd: HWND, data: LPARAM) -> i32 {
    let mut title = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
    if String::from_utf16_lossy(&title[..len.max(0) as usize]).starts_with("Primitive World ") {
        // SAFETY: EnumWindows owns this live vector for the synchronous traversal.
        unsafe {
            (*(data as *mut Vec<HWND>)).push(hwnd);
        }
    }
    1
}

unsafe extern "system" fn tray_window_proc(
    window: HWND,
    message: u32,
    wparam: usize,
    lparam: LPARAM,
) -> isize {
    if message == TRAY_CALLBACK && lparam as u32 == WM_RBUTTONUP {
        let menu = unsafe { CreatePopupMenu() };
        if !menu.is_null() {
            let pause = wide("Pause / Resume");
            let quit = wide("Quit Primitive World");
            // SAFETY: menu and item strings are valid for this call.
            unsafe {
                AppendMenuW(menu, MF_STRING, TRAY_PAUSE as usize, pause.as_ptr());
                AppendMenuW(menu, MF_STRING, TRAY_QUIT as usize, quit.as_ptr());
                let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
                GetCursorPos(&mut point);
                SetForegroundWindow(window);
                let command = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    0,
                    window,
                    null_mut(),
                );
                if command == TRAY_PAUSE as i32 {
                    TRAY_ACTION.store(TRAY_PAUSE, Ordering::Release);
                } else if command == TRAY_QUIT as i32 {
                    TRAY_ACTION.store(TRAY_QUIT, Ordering::Release);
                }
                DestroyMenu(menu);
            }
        }
        return 0;
    }
    if message == TRAY_CALLBACK && lparam as u32 == WM_LBUTTONDBLCLK {
        TRAY_ACTION.store(TRAY_PAUSE, Ordering::Release);
        return 0;
    }
    // SAFETY: unhandled messages are delegated to the standard window proc.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

impl SingleInstance {
    pub fn acquire() -> Result<Option<Self>, String> {
        let name = wide(MUTEX_NAME);
        // SAFETY: the name is a valid, nul-terminated UTF-16 string and the
        // returned handle is owned by this guard until Drop.
        let handle = unsafe { CreateMutexW(null_mut(), 1, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "Could not create the single-instance guard ({})",
                unsafe { GetLastError() }
            ));
        }
        // SAFETY: GetLastError is read immediately after CreateMutexW.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            // SAFETY: this process owns the handle returned by CreateMutexW.
            unsafe { CloseHandle(handle) };
            return Ok(None);
        }
        Ok(Some(Self { handle }))
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: the guard owns this mutex handle.
        unsafe { CloseHandle(self.handle) };
    }
}

pub fn install_startup() -> Result<String, String> {
    let executable =
        std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;
    let command = format!("{} --wallpaper --resume", quote_windows(&executable));
    let key_path = wide(STARTUP_KEY);
    let value_name = wide(STARTUP_VALUE);
    let value = wide(&command);
    let mut key: HKEY = null_mut();
    // SAFETY: all pointers refer to live nul-terminated UTF-16 buffers and
    // the output key is initialized by the call.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null_mut(),
            &mut key,
            null_mut(),
        )
    };
    if status != 0 {
        return Err(format!(
            "Could not open Windows startup settings ({status})"
        ));
    }
    // SAFETY: key is valid after the successful RegCreateKeyExW call; value
    // is UTF-16 including its terminating nul.
    let status = unsafe {
        RegSetValueExW(
            key,
            value_name.as_ptr(),
            0,
            REG_SZ,
            value.as_ptr().cast(),
            (value.len() * 2) as u32,
        )
    };
    // SAFETY: key is valid and must be closed exactly once.
    unsafe { RegCloseKey(key) };
    if status != 0 {
        return Err(format!(
            "Could not register Windows startup entry ({status})"
        ));
    }
    Ok(format!(
        "Installed Primitive World for Windows login: {command}"
    ))
}

pub fn uninstall_startup() -> Result<String, String> {
    let key_path = wide(STARTUP_KEY);
    let value_name = wide(STARTUP_VALUE);
    let mut key: HKEY = null_mut();
    // SAFETY: buffers are valid UTF-16 strings and the output key is written
    // by the call.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null_mut(),
            &mut key,
            null_mut(),
        )
    };
    if status != 0 {
        return Err(format!(
            "Could not open Windows startup settings ({status})"
        ));
    }
    // SAFETY: key and value_name are valid handles/pointers.
    let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
    // SAFETY: key is valid and must be closed exactly once.
    unsafe { RegCloseKey(key) };
    if status != 0 && status != 2 {
        return Err(format!("Could not remove Windows startup entry ({status})"));
    }
    Ok("Removed Primitive World from Windows login startup".into())
}

pub fn attach_to_desktop(window: &Window) -> Result<(), String> {
    let hwnd =
        window_handle(window).ok_or("Primitive World did not expose a Windows window handle")?;
    let host = desktop_host(true)?;
    let worker = host.parent;
    // SAFETY: both handles come from Windows and are valid while the windows
    // are alive. The wallpaper is an Explorer child; the selected host also
    // describes where it belongs relative to the icon and background layers.
    // Windows requires its style to reflect that relationship before reparenting.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (style & !(WS_POPUP as isize)) | WS_CHILD as isize,
        );
        SetParent(hwnd, worker);
        if GetParent(hwnd) != worker {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style);
            return Err(format!(
                "Could not parent wallpaper to desktop ({})",
                GetLastError()
            ));
        }
        let extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if host.layered {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, extended | WS_EX_LAYERED as isize);
            if SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA) == 0 {
                return Err(format!(
                    "Could not enable wallpaper composition ({}, extended style={:#x})",
                    GetLastError(),
                    GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
                ));
            }
        } else if extended & WS_EX_LAYERED as isize != 0 {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, extended & !(WS_EX_LAYERED as isize));
        }
        let mut rect: RECT = std::mem::zeroed();
        if GetClientRect(worker, &mut rect) == 0 {
            return Err("Could not read the desktop background bounds".into());
        }
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return Err("Desktop background has empty bounds".into());
        }
        if SetWindowPos(
            hwnd,
            host.insert_after,
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_NOSENDCHANGING | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        ) == 0
        {
            return Err(format!(
                "Could not size wallpaper to desktop ({})",
                GetLastError()
            ));
        }
        eprintln!(
            "Wallpaper attached: child={hwnd:p}, parent={worker:p}, bounds={}x{}, layout={}, below={:p}",
            rect.right - rect.left,
            rect.bottom - rect.top,
            if host.layered {
                "raised desktop"
            } else {
                "legacy WorkerW"
            },
            host.insert_after,
        );
        WALLPAPER_WINDOW.store(hwnd as isize, Ordering::Release);
    }
    Ok(())
}

struct DesktopHost {
    parent: HWND,
    insert_after: HWND,
    layered: bool,
}

fn desktop_host(create: bool) -> Result<DesktopHost, String> {
    let progman = wide("Progman");
    // SAFETY: class name is a valid UTF-16 string. The message asks Explorer
    // to ensure the desktop worker window exists.
    let progman = unsafe { FindWindowW(progman.as_ptr(), null_mut()) };
    if progman.is_null() {
        return Err("Windows desktop window (Progman) was not found".into());
    }
    let mut result = 0;
    // SAFETY: progman is a live window and the timeout prevents blocking on a
    // misbehaving shell extension.
    if create {
        unsafe {
            SendMessageTimeoutW(
                progman,
                DESKTOP_MESSAGE,
                DESKTOP_MESSAGE_WPARAM,
                DESKTOP_MESSAGE_LPARAM,
                SMTO_NORMAL,
                1000,
                &mut result,
            );
        }
    }
    let shell = wide("SHELLDLL_DefView");
    let worker_class = wide("WorkerW");
    // Raised Windows 11 desktops keep both icons and the normal background
    // under Progman. Our opaque layered child belongs directly below DefView,
    // ABOVE the background WorkerW; HWND_BOTTOM would hide it behind that image.
    let shell_view = unsafe { FindWindowExW(progman, null_mut(), shell.as_ptr(), null_mut()) };
    let background =
        unsafe { FindWindowExW(progman, null_mut(), worker_class.as_ptr(), null_mut()) };
    if !shell_view.is_null() && !background.is_null() {
        return Ok(DesktopHost {
            parent: progman,
            insert_after: shell_view,
            layered: true,
        });
    }
    let mut data = WorkerSearch { worker: null_mut() };
    // SAFETY: callback and lparam remain valid for the duration of EnumWindows.
    unsafe { EnumWindows(Some(find_worker), &mut data as *mut _ as LPARAM) };
    if !data.worker.is_null() {
        Ok(DesktopHost {
            parent: data.worker,
            insert_after: HWND_BOTTOM,
            layered: false,
        })
    } else {
        Err("Explorer did not expose a supported wallpaper layer; refusing an invisible Progman fallback".into())
    }
}

struct WorkerSearch {
    worker: HWND,
}

unsafe extern "system" fn find_worker(hwnd: HWND, lparam: LPARAM) -> i32 {
    let shell = wide("SHELLDLL_DefView");
    // SAFETY: hwnd is supplied by EnumWindows and the class buffer is valid.
    let shell_view = unsafe { FindWindowExW(hwnd, null_mut(), shell.as_ptr(), null_mut()) };
    if !shell_view.is_null() {
        let worker_name = wide("WorkerW");
        // SAFETY: searching the top-level window list with the current hwnd
        // as the insertion point is the documented WorkerW discovery pattern.
        let worker = unsafe { FindWindowExW(null_mut(), hwnd, worker_name.as_ptr(), null_mut()) };
        if !worker.is_null() && unsafe { IsWindowVisible(worker) } != 0 {
            // SAFETY: lparam is the WorkerSearch pointer supplied above.
            unsafe { (*(lparam as *mut WorkerSearch)).worker = worker };
            return 0;
        }
    }
    1
}

fn window_handle(window: &Window) -> Option<HWND> {
    let handle = window.window_handle().ok()?.as_raw();
    match handle {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut c_void),
        _ => None,
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn quote_windows(path: &std::path::Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an interactive Windows desktop with some empty desktop exposed"]
    fn desktop_accessibility_hit_test_smoke() {
        use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .unwrap();
        let mut blank = 0;
        let mut other = 0;
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        for y in (20..height).step_by(100) {
            for x in (20..width).step_by(100) {
                let point = windows_sys::Win32::Foundation::POINT { x, y };
                if desktop_background_was_clicked(point) {
                    blank += 1;
                    // Repeat the actual cross-process query that used to crash Explorer.
                    for _ in 0..3 {
                        assert!(desktop_background_was_clicked(point));
                    }
                } else {
                    other += 1;
                }
            }
        }
        unsafe { CoUninitialize() };
        println!("Desktop hit tests: {blank} empty points accepted, {other} other points rejected");
        assert!(
            blank > 0,
            "Expose some empty desktop before running this test"
        );
        assert!(
            other > 0,
            "Expected icons, taskbar, or application windows to be rejected"
        );
    }
}
