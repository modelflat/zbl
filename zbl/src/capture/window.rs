use std::{
    collections::HashMap,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        RwLock,
    },
};

use once_cell::sync::Lazy;
use windows::{
    core::{Result, BOOL},
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::{HWND, LPARAM, POINT, RECT},
        Graphics::{
            Direct3D11::D3D11_BOX,
            Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL},
            Gdi::ClientToScreen,
        },
        System::{
            Console::GetConsoleWindow, WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
        },
        UI::{
            Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
            WindowsAndMessaging::{
                EnumWindows, GetAncestor, GetClassNameW, GetClientRect, GetShellWindow,
                GetWindowLongW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
                IsWindowVisible, EVENT_OBJECT_DESTROY, GA_ROOT, GWL_EXSTYLE, GWL_STYLE,
                WINEVENT_OUTOFCONTEXT, WS_DISABLED, WS_EX_TOOLWINDOW,
            },
        },
    },
};

use crate::util::convert_u16_string;

use super::Capturable;

static OBJECT_DESTROYED_USER_DATA: Lazy<RwLock<HashMap<isize, (isize, SyncSender<()>)>>> =
    Lazy::new(Default::default);

extern "system" fn object_destroyed_cb(
    this: HWINEVENTHOOK,
    _: u32,
    handle: HWND,
    id_object: i32,
    id_child: i32,
    _: u32,
    _: u32,
) {
    if id_object == 0 && id_child == 0 && handle != HWND::default() {
        let has_been_closed = if let Ok(handles) = OBJECT_DESTROYED_USER_DATA.read() {
            if let Some((window_handle, tx)) = handles.get(&(this.0 as isize)) {
                if *window_handle == handle.0 as isize {
                    tx.send(()).ok();
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // TODO is that correct?
            true
        };

        if has_been_closed {
            unsafe {
                let _ = UnhookWinEvent(this);
            }
        }
    }
}

extern "system" fn enum_windows_cb(window: HWND, state: LPARAM) -> BOOL {
    let window_info = Window::new(window);
    if window_info.is_capturable() {
        let state = unsafe { Box::leak(Box::from_raw(state.0 as *mut Vec<Window>)) };
        state.push(window_info);
    }
    true.into()
}

fn enumerate_capturable_windows() -> Vec<Window> {
    let state = Box::into_raw(Box::default());
    *unsafe {
        EnumWindows(Some(enum_windows_cb), LPARAM(state as isize)).expect("EnumWindows");
        Box::from_raw(state)
    }
}

fn find_window_by_name(window_name: &str) -> Vec<Window> {
    let mut found: Vec<Window> = Vec::new();
    let name_lower = window_name.to_lowercase();
    for window_info in enumerate_capturable_windows() {
        if window_info.title.to_lowercase().contains(&name_lower) {
            found.push(window_info.clone());
        }
    }
    found
}

fn get_window_text(handle: HWND) -> String {
    let mut title = [0u16; 512];
    // TODO: check errors
    unsafe { GetWindowTextW(handle, &mut title) };
    convert_u16_string(&title)
}

fn get_window_class_name(handle: HWND) -> String {
    let mut class_name = [0u16; 512];
    // TODO: check errors
    unsafe { GetClassNameW(handle, &mut class_name) };
    convert_u16_string(&class_name)
}

#[derive(Clone, Debug)]
pub struct Window {
    pub handle: HWND,
    pub title: String,
    pub class_name: String,
}

impl Window {
    pub fn new(handle: HWND) -> Self {
        let title = get_window_text(handle);
        let class_name = get_window_class_name(handle);
        Self {
            handle,
            title,
            class_name,
        }
    }

    pub fn find_first(window_name: &str) -> Option<Window> {
        find_window_by_name(window_name).into_iter().next()
    }

    pub fn matches_title_and_class_name(&self, title: &str, class_name: &str) -> bool {
        self.title == title && self.class_name == class_name
    }

    pub fn is_known_blocked_window(&self) -> bool {
        // Task View
        self.matches_title_and_class_name("Task View", "Windows.UI.Core.CoreWindow") ||
        // XAML Islands
        self.matches_title_and_class_name("DesktopWindowXamlSource", "Windows.UI.Core.CoreWindow") ||
        // XAML Popups
        self.matches_title_and_class_name("PopupHost", "Xaml_WindowedPopupClass")
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.handle).as_bool() }
    }

    pub fn is_shell_window(&self) -> bool {
        self.handle == unsafe { GetShellWindow() }
    }

    pub fn is_console_window(&self) -> bool {
        self.handle == unsafe { GetConsoleWindow() }
    }

    pub fn get_root(&self) -> HWND {
        unsafe { GetAncestor(self.handle, GA_ROOT) }
    }

    pub fn is_top_level(&self) -> bool {
        self.get_root() == self.handle
    }

    /// https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles
    pub fn get_style(&self) -> i32 {
        unsafe { GetWindowLongW(self.handle, GWL_STYLE) }
    }

    /// https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
    pub fn get_ex_style(&self) -> i32 {
        unsafe { GetWindowLongW(self.handle, GWL_EXSTYLE) }
    }

    pub fn is_disabled(&self) -> bool {
        self.get_style() & (WS_DISABLED.0 as i32) == 1
    }

    pub fn is_tooltip(&self) -> bool {
        self.get_ex_style() & (WS_EX_TOOLWINDOW.0 as i32) == 1
    }

    pub fn is_uwp_window(&self) -> bool {
        self.class_name == "Windows.UI.Core.CoreWindow"
            || self.class_name == "ApplicationFrameWindow"
    }

    pub fn is_dwm_cloaked(&self) -> bool {
        let mut cloaked: u32 = 0;
        let dwm_attr_cloaked = unsafe {
            DwmGetWindowAttribute(
                self.handle,
                DWMWA_CLOAKED,
                &mut cloaked as *mut _ as *mut _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        dwm_attr_cloaked.is_ok() && cloaked == DWM_CLOAKED_SHELL
    }

    pub fn is_capturable(&self) -> bool {
        if !self.is_visible()
            || self.is_shell_window()
            || self.is_console_window()
            || !self.is_top_level()
            || self.is_disabled()
            || self.is_tooltip()
            || self.is_known_blocked_window()
        {
            return false;
        }

        // Check to see if the self is cloaked if it's a UWP
        if self.is_uwp_window() && self.is_dwm_cloaked() {
            return false;
        }

        true
    }

    pub fn get_process_id(&self) -> u32 {
        let mut process_id = 0u32;
        unsafe { GetWindowThreadProcessId(self.handle, Some(&mut process_id)) };
        process_id
    }

    pub fn print_info(&self) {
        println!("title = {}", self.title);
        println!("class = {}", self.class_name);
        println!("is_capturable = {}", self.is_capturable());
        println!("\tis_visible = {}", self.is_visible());
        println!("\tis_shell_window = {}", self.is_shell_window());
        println!("\tis_console_window = {}", self.is_console_window());
        println!("\tis_top_level = {}", self.is_top_level());
        println!("\tis_disabled = {}", self.is_disabled());
        println!("\tis_tooltip = {}", self.is_tooltip());
        println!("\tis_uwp_window = {}", self.is_uwp_window());
        println!("\tis_dwm_cloaked = {}", self.is_dwm_cloaked());
        println!(
            "\tis_known_blocked_window = {}",
            self.is_known_blocked_window()
        );
    }
}

impl Capturable for Window {
    fn create_capture_item(&self) -> Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        unsafe { interop.CreateForWindow(self.handle) }
    }

    fn get_client_box(&self) -> Result<D3D11_BOX> {
        let mut window_rect = RECT::default();
        let mut client_rect = RECT::default();
        let mut top_left = POINT::default();
        unsafe {
            GetWindowRect(self.handle, &mut window_rect)?;
            let _ = ClientToScreen(self.handle, &mut top_left);
            GetClientRect(self.handle, &mut client_rect)?;
        }

        let mut client_box = D3D11_BOX::default();
        // TODO
        // 1 seems to work because most window have a 1-pixel gap in the D3D11 texture
        // produced by Windows.Graphics.Capture. Why tho?
        client_box.left = 1;
        client_box.right = client_box.left + (client_rect.right - client_rect.left) as u32;
        // TODO there seems to be no reliadble way of getting the taskbar height, so this code is fairly brittle
        client_box.top = (top_left.y - window_rect.top) as u32;
        client_box.bottom = client_box.top + (client_rect.bottom - client_rect.top) as u32;
        client_box.front = 0;
        client_box.back = 1;
        Ok(client_box)
    }

    fn get_close_notification_channel(&self) -> Receiver<()> {
        let (sender, receiver) = sync_channel(1);
        let hook_id = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_DESTROY,
                None,
                Some(object_destroyed_cb),
                // TODO filtering by process id does not always catch the moment when the window is closed
                // why? aren't windows bound to their process ids?
                // moreover, for explorer windows even that does not work.
                // need some more realiable and simpler way to track window closing
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if let Ok(mut handles) = OBJECT_DESTROYED_USER_DATA.write() {
            handles.insert(hook_id.0 as isize, (self.handle.0 as isize, sender));
        }
        receiver
    }

    fn get_raw_handle(&self) -> isize {
        self.handle.0 as isize
    }
}
