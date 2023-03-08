use windows::core::Result;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Direct3D11::D3D11_BOX;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Console::GetConsoleWindow;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClientRect, GetShellWindow, GetWindowLongW, GetWindowRect,
    GetWindowThreadProcessId, IsWindowVisible, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, WS_DISABLED,
    WS_EX_TOOLWINDOW,
};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindowTextW};

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
        EnumWindows(Some(enum_windows_cb), LPARAM(state as isize));
        Box::from_raw(state)
    }
}

fn find_by_name(window_name: &str) -> Vec<Window> {
    let mut found: Vec<Window> = Vec::new();
    let name_lower = window_name.to_lowercase();
    for window_info in enumerate_capturable_windows() {
        if window_info.title.to_lowercase().contains(&name_lower) {
            found.push(window_info.clone());
        }
    }
    found
}

fn convert_u16_string(input: &[u16; 512]) -> String {
    let mut s = String::from_utf16_lossy(input);
    if let Some(index) = s.find('\0') {
        s.truncate(index);
    }
    s
}

#[derive(Clone, Debug)]
pub struct Window {
    pub handle: HWND,
    pub title: String,
    pub class_name: String,
}

impl Window {
    pub fn new(handle: HWND) -> Self {
        // TODO: check errors
        let title = {
            let mut title = [0u16; 512];
            unsafe { GetWindowTextW(handle, &mut title) };
            convert_u16_string(&title)
        };

        // TODO: check errors
        let class_name = {
            let mut class_name = [0u16; 512];
            unsafe { GetClassNameW(handle, &mut class_name) };
            convert_u16_string(&class_name)
        };

        Self {
            handle,
            title,
            class_name,
        }
    }

    pub fn find_first(window_name: &str) -> Option<Window> {
        find_by_name(window_name).into_iter().next()
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

    pub fn is_capturable(&self) -> bool {
        unsafe {
            if self.title.is_empty()
                || self.handle == GetShellWindow()
                || self.handle == GetConsoleWindow()
                || !IsWindowVisible(self.handle).as_bool()
                || GetAncestor(self.handle, GA_ROOT) != self.handle
            {
                return false;
            }
        }

        let style = unsafe { GetWindowLongW(self.handle, GWL_STYLE) };
        if style & (WS_DISABLED.0 as i32) == 1 {
            return false;
        }

        // No tooltips
        let ex_style = unsafe { GetWindowLongW(self.handle, GWL_EXSTYLE) };
        if ex_style & (WS_EX_TOOLWINDOW.0 as i32) == 1 {
            return false;
        }

        // Unfortunate work-around. Not sure how to avoid this.
        if self.is_known_blocked_window() {
            return false;
        }

        // Check to see if the self is cloaked if it's a UWP
        if self.class_name == "Windows.UI.Core.CoreWindow"
            || self.class_name == "ApplicationFrameWindow"
        {
            let mut cloaked: u32 = 0;
            let dwm_attr_cloaked = unsafe {
                DwmGetWindowAttribute(
                    self.handle,
                    DWMWA_CLOAKED,
                    &mut cloaked as *mut _ as *mut _,
                    std::mem::size_of::<u32>() as u32,
                )
            };
            if dwm_attr_cloaked.is_ok() && cloaked == DWM_CLOAKED_SHELL {
                return false;
            }
        }

        true
    }

    pub fn get_process_id(&self) -> u32 {
        let mut process_id = 0u32;
        unsafe { GetWindowThreadProcessId(self.handle, Some(&mut process_id as *mut _)) };
        process_id
    }

    pub fn create_capture_item(&self) -> Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        unsafe { interop.CreateForWindow(self.handle) }
    }

    pub fn get_client_box(&self) -> D3D11_BOX {
        let mut window_rect = RECT::default();
        let mut client_rect = RECT::default();
        let mut top_left = POINT::default();
        unsafe {
            GetWindowRect(self.handle, &mut window_rect as *mut _);
            ClientToScreen(self.handle, &mut top_left as *mut _);
            GetClientRect(self.handle, &mut client_rect as *mut _);
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
        client_box
    }
}
