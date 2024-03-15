pub mod display;
pub mod window;

use std::sync::mpsc::{sync_channel, Receiver, TryRecvError, TrySendError};

use windows::{
    core::{IInspectable, Interface, Result},
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
        Graphics::Direct3D11::{
            ID3D11Texture2D, D3D11_BOX, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
        },
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};

use crate::{d3d::D3D, frame::Frame};

pub trait Capturable {
    fn create_capture_item(&self) -> Result<GraphicsCaptureItem>;

    fn get_client_box(&self) -> Result<D3D11_BOX>;

    fn get_close_notification_channel(&self) -> Receiver<()>;

    fn get_raw_handle(&self) -> isize;
}

pub struct Capture {
    d3d: D3D,
    capturable: Box<dyn Capturable>,
    capture_box: D3D11_BOX,
    capture_done_signal: Receiver<()>,
    frame_pool: Direct3D11CaptureFramePool,
    frame_source: Receiver<Option<Direct3D11CaptureFrame>>,
    session: GraphicsCaptureSession,
    use_staging_texture: bool,
    staging_texture: Option<ID3D11Texture2D>,
    content_size: SizeInt32,
    stopped: bool,
}

impl Capture {
    /// Create a new capture. This will initialize D3D11 devices, context, and Windows.Graphics.Capture's
    /// frame pool / capture session.
    ///
    /// Note that this will not start capturing yet. Call `start()` to actually start receiving frames.
    pub fn new(
        capturable: Box<dyn Capturable>,
        capture_cursor: bool,
        use_staging_texture: bool,
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
        session.SetIsCursorCaptureEnabled(capture_cursor)?;

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
            use_staging_texture,
            staging_texture: None,
            content_size: Default::default(),
            stopped: false,
        })
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
    /// * `Ok(None)` if no frames can be received (e.g. when the window was closed).
    /// * `Err(...)` if an error has occured while capturing a frame.
    pub fn grab(&mut self) -> Result<Option<Frame>> {
        match self.receive_next_frame()? {
            Some(frame) => {
                let texture: ID3D11Texture2D = get_dxgi_interface_from_object(&frame.Surface()?)?;
                if self.use_staging_texture {
                    self.copy_to_staging(&texture)?;
                    let texture = self
                        .staging_texture
                        .clone()
                        .expect("staging texture should be initialized at this point");
                    let ptr = self.d3d.map_unmap_texture(&texture)?;
                    Ok(Some(Frame::new(texture, ptr)))
                } else {
                    Ok(Some(Frame::new(
                        texture,
                        D3D11_MAPPED_SUBRESOURCE::default(),
                    )))
                }
            }
            None => Ok(None),
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

    fn receive_next_frame(&mut self) -> Result<Option<Direct3D11CaptureFrame>> {
        if self.stopped {
            return Ok(None);
        }
        loop {
            match self.frame_source.try_recv() {
                Ok(Some(f)) => return Ok(Some(f)),
                Err(TryRecvError::Empty) => {
                    // TODO busy loop? so uncivilized
                    if let Ok(()) | Err(TryRecvError::Disconnected) =
                        self.capture_done_signal.try_recv()
                    {
                        self.stop()?;
                        return Ok(None);
                    }
                }
                Ok(None) | Err(TryRecvError::Disconnected) => return Ok(None),
            }
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
                self.use_staging_texture,
            )?;
            self.staging_texture = Some(new_staging_texture);
            self.content_size = content_size;
        }

        self.d3d.copy_texture(
            frame_texture,
            self.staging_texture.as_ref().unwrap(),
            &self.capture_box,
        )?;

        // TODO queue a fence here? currently we ensure buffer is copied by map-unmap texture outside of this method,
        // which is probably not the best way to do this

        Ok(())
    }
}

fn get_dxgi_interface_from_object<S: Interface, R: Interface>(object: &S) -> Result<R> {
    let access: IDirect3DDxgiInterfaceAccess = object.cast()?;
    let object = unsafe { access.GetInterface::<R>()? };
    Ok(object)
}
