pub mod capture;
pub mod d3d;
pub mod frame;
pub mod util;

pub use capture::{Capturable, Capture, CaptureBuilder, display::Display, window::Window};
pub use frame::Frame;

// re-export winapi
pub use windows;

use std::sync::LazyLock;
use windows::Win32::{
    System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize},
    UI::HiDpi::{PROCESS_PER_MONITOR_DPI_AWARE, SetProcessDpiAwareness},
};

pub fn init() {
    ro_initialize_once();
    set_dpi_aware();
}

static STATE: LazyLock<()> = LazyLock::new(ro_initialize);

pub fn ro_initialize_once() {
    *STATE
}

pub fn ro_initialize() {
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED).ok();
    }
}

pub fn set_dpi_aware() {
    unsafe {
        SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE).ok();
    }
}
