pub mod capture;
pub mod staging_texture;
pub mod util;
pub mod window;

pub use capture::Capture;
pub use window::Window;

// re-export winapi
pub use windows;

use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::{
    System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
    UI::HiDpi::{SetProcessDpiAwareness, PROCESS_SYSTEM_DPI_AWARE},
};

pub fn init() {
    ro_initialize_once();
    set_dpi_aware();
}

pub fn ro_initialize_once() {
    static mut STATE: AtomicBool = AtomicBool::new(false);
    unsafe {
        let state = STATE.swap(true, Ordering::SeqCst);
        if !state {
            RoInitialize(RO_INIT_MULTITHREADED).ok();
        }
    };
}

pub fn set_dpi_aware() {
    unsafe {
        SetProcessDpiAwareness(PROCESS_SYSTEM_DPI_AWARE).ok();
    }
}
