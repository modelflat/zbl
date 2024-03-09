pub mod capture;
pub mod d3d;
pub mod staging_texture;
pub mod util;

pub use capture::display::Display;
pub use capture::window::Window;
pub use capture::{Capture, Frame};

// re-export winapi
pub use windows;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
};
use windows::{
    core::Result,
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Graphics::Direct3D11::D3D11_BOX,
        System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
        UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE},
    },
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
        SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE).ok();
    }
}

pub trait Capturable {
    fn create_capture_item(&self) -> Result<GraphicsCaptureItem>;

    fn get_client_box(&self) -> Result<D3D11_BOX>;

    fn get_close_notification_channel(&self) -> Receiver<()>;

    fn get_raw_handle(&self) -> isize;
}
