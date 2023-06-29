use std::{
    collections::HashMap,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        RwLock,
    },
};

use lazy_static::lazy_static;
use windows::{
    core::Result,
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::{BOOL, LPARAM, RECT},
        Graphics::{
            Direct3D11::D3D11_BOX,
            Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
        },
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
};

use crate::{util::convert_u16_string, Capturable};

lazy_static! {
    static ref OBJECT_DESTROYED_USER_DATA: RwLock<HashMap<isize, (isize, SyncSender<()>)>> =
        Default::default();
}

fn get_monitor_info(handle: HMONITOR) -> Result<MONITORINFOEXW> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    unsafe {
        GetMonitorInfoW(handle, &mut info as *mut _ as *mut _).ok()?;
    }
    Ok(info)
}

extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<Result<Display>>));
        state.push(Display::new(monitor));
    }
    true.into()
}

fn enumerate_displays() -> Result<Box<Vec<Result<Display>>>> {
    let displays = Box::into_raw(Default::default());
    unsafe {
        EnumDisplayMonitors(HDC(0), None, Some(enum_monitor), LPARAM(displays as isize)).ok()?;
        Ok(Box::from_raw(displays))
    }
}

#[derive(Clone, Debug)]
pub struct Display {
    pub handle: HMONITOR,
    pub display_name: String,
    pub display_info: MONITORINFOEXW,
}

impl Display {
    pub fn new(handle: HMONITOR) -> Result<Self> {
        let display_info = get_monitor_info(handle)?;
        let display_name = convert_u16_string(&display_info.szDevice);
        Ok(Self {
            handle,
            display_name,
            display_info,
        })
    }

    pub fn find_by_id(id: usize) -> Result<Self> {
        let displays = *enumerate_displays()?;
        displays[id].clone()
    }

    pub fn get_virtual_size(&self) -> (i32, i32) {
        let rect = self.display_info.monitorInfo.rcMonitor;
        (rect.right - rect.left, rect.bottom - rect.top)
    }
}

impl Capturable for Display {
    fn create_capture_item(&self) -> Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        unsafe { interop.CreateForMonitor(self.handle) }
    }

    fn get_client_box(&self) -> Result<D3D11_BOX> {
        let (w, h) = self.get_virtual_size();

        let mut client_box = D3D11_BOX::default();
        client_box.left = 0;
        client_box.right = w as u32;
        client_box.top = 0;
        client_box.bottom = h as u32;
        client_box.front = 0;
        client_box.back = 1;
        Ok(client_box)
    }

    fn get_close_notification_channel(&self) -> Receiver<()> {
        let (sender, receiver) = sync_channel(1);
        OBJECT_DESTROYED_USER_DATA
            .write()
            .unwrap()
            .insert(self.handle.0, (self.handle.0, sender));
        receiver
    }

    fn get_raw_handle(&self) -> isize {
        self.handle.0
    }
}
