pub mod display;
pub mod window;

use std::sync::mpsc::{Receiver, TryRecvError, TrySendError, sync_channel};

use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::DirectXPixelFormat,
        SizeInt32,
    },
    Win32::{
        Graphics::Direct3D11::{D3D11_BOX, D3D11_TEXTURE2D_DESC, ID3D11Texture2D},
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
    core::{IInspectable, Interface, Result},
};

use crate::{d3d::D3D, frame::Frame};

pub trait Capturable {
    fn create_capture_item(&self) -> Result<GraphicsCaptureItem>;

    fn get_client_box(&self) -> Result<D3D11_BOX>;

    fn get_close_notification_channel(&self) -> Receiver<()>;

    fn get_raw_handle(&self) -> isize;
}

pub enum MaybeFrame {
    Some(Frame),
    Pending,
    None,
}

pub struct CaptureBuilder {
    capturable: Box<dyn Capturable>,
    is_cursor_capture_enabled: bool,
    is_border_required: bool,
    cpu_access: bool,
}

impl CaptureBuilder {
    pub fn new(capturable: Box<dyn Capturable>) -> Self {
        Self {
            capturable,
            is_cursor_capture_enabled: false,
            is_border_required: true,
            cpu_access: true,
        }
    }

    pub fn set_is_cursor_capture_enabled(mut self, val: bool) -> Self {
        self.is_cursor_capture_enabled = val;
        self
    }

    pub fn set_is_border_required(mut self, val: bool) -> Self {
        self.is_border_required = val;
        self
    }

    pub fn set_cpu_access(mut self, val: bool) -> Self {
        self.cpu_access = val;
        self
    }

    pub fn build(self) -> Result<Capture> {
        Capture::new(
            self.capturable,
            self.is_cursor_capture_enabled,
            self.is_border_required,
            self.cpu_access,
        )
    }
}

/// Represents a Capture session.
pub struct Capture {
    d3d: D3D,
    capturable: Box<dyn Capturable>,
    capture_box: D3D11_BOX,
    capture_done_signal: Receiver<()>,
    frame_pool: Direct3D11CaptureFramePool,
    frame_source: Receiver<Option<Direct3D11CaptureFrame>>,
    session: GraphicsCaptureSession,
    cpu_access: bool,
    staging_texture: Option<ID3D11Texture2D>,
    content_size: SizeInt32,
    stopped: bool,
}

impl Capture {
    /// Create a new capture. This will initialize D3D11 devices, context, and Windows.Graphics.Capture's
    /// frame pool / capture session.
    ///
    /// Note that this will not start capturing yet. Call `start()` to actually start receiving frames.
    pub(crate) fn new(
        capturable: Box<dyn Capturable>,
        is_cursor_capture_enabled: bool,
        is_border_required: bool,
        cpu_access: bool,
    ) -> Result<Self> {
        let d3d = D3D::new()?;
        let capture_item = capturable.create_capture_item()?;
        let capture_item_size = capture_item.Size()?;

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &d3d.direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            capture_item_size,
        )?;

        let session = frame_pool.CreateCaptureSession(&capture_item)?;
        session.SetIsCursorCaptureEnabled(is_cursor_capture_enabled)?;
        if !is_border_required {
            if let Err(e) = session.SetIsBorderRequired(is_border_required) {
                log::warn!(
                    "got '{}' when trying to disable the capture border - see https://github.com/modelflat/zbl/pull/4 for more info",
                    e
                );
            }
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
                            log::info!("dropping frame {}", ts.Duration);
                        }
                        Err(TrySendError::Disconnected(_)) => {
                            log::info!("frame receiver disconnected");
                        }
                        _ => {}
                    }
                    Ok(())
                },
            ),
        )?;

        let capture_box = capturable.get_client_box()?;
        let capture_done_signal = capturable.get_close_notification_channel();

        Ok(Self {
            d3d,
            capturable,
            capture_box,
            capture_done_signal,
            frame_pool,
            frame_source: receiver,
            session,
            cpu_access,
            staging_texture: None,
            content_size: Default::default(),
            stopped: false,
        })
    }

    /// Get D3D contexts
    pub fn d3d(&mut self) -> &mut D3D {
        &mut self.d3d
    }

    /// Whether the backing buffer of this instance of `Capture` is CPU-accessible.
    pub fn has_cpu_access(&self) -> bool {
        self.cpu_access
    }

    /// Get attached capturable.
    pub fn capturable(&self) -> &dyn Capturable {
        self.capturable.as_ref()
    }

    /// Start capturing frames.
    pub fn start(&self) -> Result<()> {
        self.session.StartCapture()
    }

    /// Grab current capture frame.
    ///
    /// **This method blocks if there is no frames in the frame pool** (happens when application's window
    /// is minimized, for example).
    ///
    /// Returns:
    /// * `Ok(Some(...))` if there is a frame and it's been successfully captured;
    /// * `Ok(None)` if no frames can be received anymore (e.g. when the window was closed).
    /// * `Err(...)` if an error has occured while capturing a frame.
    pub fn grab(&mut self) -> Result<Option<Frame>> {
        loop {
            match self.try_grab()? {
                MaybeFrame::Some(f) => return Ok(Some(f)),
                MaybeFrame::Pending => {}
                MaybeFrame::None => return Ok(None),
            }
        }
    }

    /// Try grabbing current capture frame.
    ///
    /// Returns:
    /// * `Ok(MaybeFrame::Some(frame))` if there is a frame and it's been successfully captured;
    /// * `Ok(MaybeFrame::Pending)` if no frames are ready yet, but the capture isn't stopped yet.
    /// * `Ok(MaybeFrame::None)` if no frames can be received anymore (e.g. when the window was closed).
    /// * `Err(...)` if an error has occured while capturing a frame.
    pub fn try_grab(&mut self) -> Result<MaybeFrame> {
        if self.stopped {
            return Ok(MaybeFrame::None);
        }
        match self.frame_source.try_recv() {
            Ok(Some(f)) => return Ok(MaybeFrame::Some(self.convert_to_frame(f)?)),
            Err(TryRecvError::Empty) => {
                if let Ok(()) | Err(TryRecvError::Disconnected) =
                    self.capture_done_signal.try_recv()
                {
                    self.stop()?;
                    Ok(MaybeFrame::None)
                } else {
                    Ok(MaybeFrame::Pending)
                }
            }
            Ok(None) | Err(TryRecvError::Disconnected) => return Ok(MaybeFrame::None),
        }
    }

    /// Stops the capture.
    ///
    /// This `Capture` instance cannot be reused after that (i.e. calling `start()` again will
    /// **not** produce more frames).
    pub fn stop(&mut self) -> Result<()> {
        self.stopped = true;
        self.session.Close()?;
        self.frame_pool.Close()?;
        Ok(())
    }

    fn needs_resize(&self, new_size: SizeInt32) -> bool {
        self.content_size.Width != new_size.Width
            || self.content_size.Height != new_size.Height
            || self.staging_texture.is_none()
    }

    fn recreate_frame_pool(&mut self) -> Result<()> {
        let capture_item = self.capturable.create_capture_item()?;
        let capture_item_size = capture_item.Size()?;
        self.capture_box = self.capturable.get_client_box()?;
        self.frame_pool.Recreate(
            &self.d3d.direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            capture_item_size,
        )?;
        Ok(())
    }

    fn convert_to_frame(&mut self, frame: Direct3D11CaptureFrame) -> Result<Frame> {
        let original_texture: ID3D11Texture2D = get_dxgi_interface_from_object(&frame.Surface()?)?;

        // TODO can we avoid copying data into staging texture when DirectX interop is enabled?
        // currently it doesn't work because of the following error:
        //   OpenCL: clCreateFromD3D11Texture2DNV failed in function 'cv::directx::__convertFromD3D11Texture2DNV'
        // which seems to be in turn caused by presence of D3D11_RESOURCE_MISC_SHARED_NTHANDLE misc flag in the
        // original frame texture
        self.copy_to_staging(&original_texture)?;

        let staging_texture = self
            .staging_texture
            .clone()
            .expect("staging texture should be initialized at this point");

        if self.cpu_access {
            let ptr = self.d3d.map_unmap_texture(&staging_texture)?;
            Ok(Frame::new_mapped(staging_texture, ptr))
        } else {
            Ok(Frame::new(staging_texture))
        }
    }

    fn copy_to_staging(&mut self, frame_texture: &ID3D11Texture2D) -> Result<()> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { frame_texture.GetDesc(&mut desc) };
        let content_size = SizeInt32 {
            Width: desc.Width as i32,
            Height: desc.Height as i32,
        };

        if self.needs_resize(content_size) {
            self.recreate_frame_pool()?;
            let new_staging_texture = self.d3d.create_texture(
                self.capture_box.right - self.capture_box.left,
                self.capture_box.bottom - self.capture_box.top,
                desc.Format,
                self.cpu_access,
            )?;
            self.staging_texture = Some(new_staging_texture);
            self.content_size = content_size;
        }

        self.d3d.copy_texture(
            frame_texture,
            self.staging_texture.as_ref().unwrap(),
            &self.capture_box,
        )?;

        Ok(())
    }
}

fn get_dxgi_interface_from_object<S: Interface, R: Interface>(object: &S) -> Result<R> {
    let access: IDirect3DDxgiInterfaceAccess = object.cast()?;
    let object = unsafe { access.GetInterface::<R>()? };
    Ok(object)
}
