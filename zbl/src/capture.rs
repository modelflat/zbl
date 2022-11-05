use std::{
    collections::HashMap,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError},
        RwLock,
    },
};

use lazy_static::lazy_static;
use windows::{
    core::{IInspectable, Interface, Result},
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureSession},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::HWND,
        Graphics::Direct3D11::{
            ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BOX,
            D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
        },
        UI::{
            Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK},
            WindowsAndMessaging::{EVENT_OBJECT_DESTROY, WINEVENT_OUTOFCONTEXT},
        },
    },
};

use crate::{
    staging_texture::StagingTexture,
    util::{create_d3d_device, create_direct3d_device, get_dxgi_interface_from_object},
    window::Window,
};

lazy_static! {
    static ref OBJECT_DESTROYED_USER_DATA: RwLock<HashMap<isize, (HWND, SyncSender<()>)>> =
        Default::default();
}

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
            if let Some((window_handle, tx)) = handles.get(&this.0) {
                if *window_handle == handle {
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
            unsafe { UnhookWinEvent(this) };
        }
    }
}

pub struct Capture {
    device: ID3D11Device,
    direct3d_device: IDirect3DDevice,
    context: ID3D11DeviceContext,
    window: Window,
    window_box: D3D11_BOX,
    window_closed_signal: Receiver<()>,
    frame_pool: Direct3D11CaptureFramePool,
    frame_source: Receiver<Option<Direct3D11CaptureFrame>>,
    session: GraphicsCaptureSession,
    staging_texture: Option<StagingTexture>,
    #[allow(unused)]
    staging_texture_ptr: Option<D3D11_MAPPED_SUBRESOURCE>,
    content_size: SizeInt32,
}

impl Capture {
    pub fn new(window: Window) -> Result<Self> {
        let d3d_device = create_d3d_device()?;
        let d3d_context = {
            let mut d3d_context = None;
            unsafe { d3d_device.GetImmediateContext(&mut d3d_context) };
            d3d_context.expect("failed to create d3d_context")
        };
        let device = create_direct3d_device(&d3d_device)?;

        let capture_item = window.create_capture_item()?;
        let capture_item_size = capture_item.Size()?;

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            capture_item_size,
        )?;

        let session = frame_pool.CreateCaptureSession(&capture_item)?;
        session.SetIsCursorCaptureEnabled(false)?;

        let (sender, window_closed_signal) = sync_channel(1);
        let hook_id = unsafe {
            SetWinEventHook(
                EVENT_OBJECT_DESTROY,
                EVENT_OBJECT_DESTROY,
                None,
                Some(object_destroyed_cb),
                window.get_process_id(),
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        };
        if let Ok(mut handles) = OBJECT_DESTROYED_USER_DATA.write() {
            handles.insert(hook_id.0, (window.handle, sender));
        }

        let (sender, receiver) = sync_channel(1 << 5);
        frame_pool.FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
                move |frame_pool, _| {
                    let frame_pool = frame_pool.as_ref().unwrap();
                    let frame = frame_pool.TryGetNextFrame()?;
                    let ts = frame.SystemRelativeTime()?;
                    match sender.try_send(Some(frame)) {
                        Err(TrySendError::Full(_)) => {
                            // TODO keep track of these frames?
                            println!("dropping frame {}", ts.Duration);
                        }
                        Err(TrySendError::Disconnected(_)) => {
                            println!("frame receiver disconnected");
                        }
                        _ => {}
                    }
                    Ok(())
                },
            ),
        )?;

        let window_box = window.get_client_box();

        Ok(Self {
            device: d3d_device,
            direct3d_device: device,
            context: d3d_context,
            window,
            window_box,
            window_closed_signal,
            frame_pool,
            frame_source: receiver,
            session,
            staging_texture: None,
            staging_texture_ptr: None,
            content_size: Default::default(),
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn start(&self) -> Result<()> {
        self.session.StartCapture()
    }

    pub fn grab(&mut self) -> Result<Option<(&StagingTexture, D3D11_MAPPED_SUBRESOURCE)>> {
        if self.grab_next()? {
            let texture = self.staging_texture.as_ref().unwrap();
            // let ptr = self.staging_texture_ptr.as_ref().unwrap();
            let ptr = self
                .staging_texture
                .as_ref()
                .unwrap()
                .as_mapped(&self.context)?;
            Ok(Some((texture, ptr.clone())))
        } else {
            Ok(None)
        }
    }

    pub fn stop(&self) -> Result<()> {
        self.session.Close()?;
        self.frame_pool.Close()?;
        Ok(())
    }

    fn recreate_frame_pool(&mut self) -> Result<()> {
        let capture_item = self.window.create_capture_item()?;
        let capture_item_size = capture_item.Size()?;
        self.window_box = self.window.get_client_box();
        self.frame_pool.Recreate(
            &self.direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            capture_item_size,
        )?;
        Ok(())
    }

    fn grab_next(&mut self) -> Result<bool> {
        let frame = loop {
            match self.frame_source.try_recv() {
                Ok(Some(f)) => break f,
                Err(TryRecvError::Empty) => {
                    // TODO busy loop? so uncivilized
                    if let Ok(()) | Err(TryRecvError::Disconnected) =
                        self.window_closed_signal.try_recv()
                    {
                        self.stop()?;
                        return Ok(false);
                    }
                }
                Ok(None) | Err(TryRecvError::Disconnected) => return Ok(false),
            }
        };

        let frame_texture: ID3D11Texture2D = get_dxgi_interface_from_object(&frame.Surface()?)?;
        let content_size = frame.ContentSize()?;

        if self.content_size.Width != content_size.Width
            || self.content_size.Height != content_size.Height
            || self.staging_texture.is_none()
        {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            unsafe { frame_texture.GetDesc(&mut desc) };
            self.recreate_frame_pool()?;
            let new_staging_texture = StagingTexture::new(
                &self.device,
                self.window_box.right - self.window_box.left,
                self.window_box.bottom - self.window_box.top,
                desc.Format,
            )?;
            // TODO is it safe to map only once considering that CopySubresourceRegion is async?
            // self.staging_texture_ptr = Some(new_staging_texture.as_mapped(&self.context)?);
            self.staging_texture = Some(new_staging_texture);
            self.content_size = content_size;
        }

        let copy_dest = self.staging_texture.as_ref().unwrap().as_resource()?;
        let copy_src = frame_texture.cast()?;
        unsafe {
            self.context.CopySubresourceRegion(
                Some(&copy_dest),
                0,
                0,
                0,
                0,
                Some(&copy_src),
                0,
                Some(&self.window_box as *const _),
            );
        }

        Ok(true)
    }
}
