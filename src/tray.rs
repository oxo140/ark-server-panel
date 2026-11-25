#[cfg(windows)]
mod native {
    use std::{
        cell::RefCell,
        ptr::null,
        ptr::null_mut,
        sync::{
            Arc,
            atomic::{AtomicIsize, Ordering},
        },
    };
    use windows_sys::Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    };
    const CALLBACK: u32 = WM_APP + 1;
    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    pub fn stop() {
        let hwnd = WINDOW.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0);
            }
        }
    }
    pub fn active() -> bool {
        WINDOW.load(Ordering::SeqCst) != 0
    }
    unsafe fn app_icon(module: HINSTANCE) -> HICON {
        let mut pixels = vec![0u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                let at = (y * 32 + x) * 4;
                let a = (7..26).contains(&y)
                    && ((x as i32 - (16 - (y as i32 - 7) / 3)).abs() < 2
                        || (x as i32 - (16 + (y as i32 - 7) / 3)).abs() < 2
                        || (y == 20 && (11..22).contains(&x)));
                let c = if a {
                    [240, 255, 248, 255]
                } else {
                    [160, 177, 33, 255]
                };
                pixels[at..at + 4].copy_from_slice(&c);
            }
        }
        unsafe { CreateIcon(module, 32, 32, 1, 32, [0u8; 128].as_ptr(), pixels.as_ptr()) }
    }
    struct State {
        url: String,
        quit: Arc<dyn Fn() + Send + Sync>,
        icon: NOTIFYICONDATAW,
        taskbar: u32,
    }
    thread_local! {static STATE:RefCell<Option<State>>=const{RefCell::new(None)};}
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        STATE.with(|cell| {
            let state = cell.borrow();
            if let Some(s) = state.as_ref() {
                unsafe {
                    if msg == WM_CLOSE {
                        Shell_NotifyIconW(NIM_DELETE, &s.icon);
                        WINDOW.store(0, Ordering::SeqCst);
                        PostQuitMessage(0);
                        return 0;
                    }
                    if msg == s.taskbar {
                        Shell_NotifyIconW(NIM_ADD, &s.icon);
                        return 0;
                    }
                    if msg == CALLBACK {
                        if l as u32 == WM_LBUTTONDBLCLK {
                            crate::supervisor::open_browser(&s.url);
                        }
                        if l as u32 == WM_RBUTTONUP {
                            let menu = CreatePopupMenu();
                            AppendMenuW(
                                menu,
                                MF_STRING,
                                1,
                                wide("Ouvrir ARK Server Panel").as_ptr(),
                            );
                            AppendMenuW(
                                menu,
                                MF_STRING,
                                2,
                                wide("Quitter le panneau (serveurs conservés)").as_ptr(),
                            );
                            let mut p = POINT::default();
                            GetCursorPos(&mut p);
                            SetForegroundWindow(hwnd);
                            let choice = TrackPopupMenu(
                                menu,
                                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                                p.x,
                                p.y,
                                0,
                                hwnd,
                                null(),
                            );
                            DestroyMenu(menu);
                            PostMessageW(hwnd, WM_NULL, 0, 0);
                            if choice == 1 {
                                crate::supervisor::open_browser(&s.url)
                            } else if choice == 2 {
                                (s.quit)()
                            }
                        }
                        return 0;
                    }
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, w, l) }
        })
    }
    pub fn start(url: String, quit: Arc<dyn Fn() + Send + Sync>) {
        std::thread::spawn(move || unsafe {
            let class = wide("ArkServerPanelTray");
            let module = GetModuleHandleW(null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: module,
                lpszClassName: class.as_ptr(),
                ..Default::default()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("ARK Server Panel").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                module,
                null(),
            );
            if hwnd.is_null() {
                return;
            }
            let mut icon = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: CALLBACK,
                hIcon: app_icon(module),
                ..Default::default()
            };
            let tip = wide("ARK Server Panel — actif");
            icon.szTip[..tip.len()].copy_from_slice(&tip);
            let taskbar = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
            if Shell_NotifyIconW(NIM_ADD, &icon) == 0 {
                eprintln!("Impossible d’ajouter l’icône ARK dans la zone de notification.");
                DestroyIcon(icon.hIcon);
                DestroyWindow(hwnd);
                return;
            }
            WINDOW.store(hwnd as isize, Ordering::SeqCst);
            STATE.with(|s| {
                *s.borrow_mut() = Some(State {
                    url,
                    quit,
                    icon,
                    taskbar,
                })
            });
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            STATE.with(|s| {
                if let Some(s) = s.borrow().as_ref() {
                    Shell_NotifyIconW(NIM_DELETE, &s.icon);
                    DestroyIcon(s.icon.hIcon);
                }
            });
            DestroyWindow(hwnd);
        });
    }
}
pub fn stop() {
    #[cfg(windows)]
    native::stop();
}
pub fn active() -> bool {
    #[cfg(windows)]
    {
        native::active()
    }
    #[cfg(not(windows))]
    {
        false
    }
}
pub fn start(url: String, quit: std::sync::Arc<dyn Fn() + Send + Sync>) {
    #[cfg(windows)]
    native::start(url, quit);
    #[cfg(not(windows))]
    let _ = (url, quit);
}
