use ::zbl::windows::Win32::Foundation::HWND;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::ffi::c_void;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("window with given name not found: {0}")]
    WindowNotFoundError(String),
    #[error("windows api error: {0}")]
    WindowsError(#[from] ::zbl::windows::core::Error),
    #[error("frame channel error")]
    FrameChannelError(#[from] std::sync::mpsc::RecvError),
    #[error("neither name nor handle is set")]
    NeitherNameNorHandleIsSet,
}

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        PyRuntimeError::new_err(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[pyclass(unsendable)]
pub struct Frame {
    width: u32,
    height: u32,
    row_pitch: u32,
    ptr: *mut c_void,
}

#[pymethods]
impl Frame {
    #[getter]
    pub fn width(&self) -> usize {
        self.width as usize
    }

    #[getter]
    pub fn height(&self) -> usize {
        self.height as usize
    }

    #[getter]
    pub fn row_pitch(&self) -> usize {
        self.row_pitch as usize
    }

    #[getter]
    pub fn ptr(&self) -> usize {
        self.ptr as usize
    }
}

#[pyclass(unsendable)]
pub struct Capture {
    inner: ::zbl::Capture,
}

impl Capture {
    pub fn from_capturable(
        capturable: Box<dyn ::zbl::Capturable>,
        capture_cursor: bool,
        use_staging_texture: bool,
    ) -> Result<Self> {
        ::zbl::init();
        let capture = ::zbl::Capture::new(capturable, capture_cursor, use_staging_texture)?;
        Ok(Self { inner: capture })
    }

    pub fn from_window_name(
        name: &str,
        capture_cursor: bool,
        use_staging_texture: bool,
    ) -> Result<Self> {
        let window = ::zbl::Window::find_first(name)
            .ok_or_else(|| Error::WindowNotFoundError(name.to_string()))?;
        Self::from_capturable(
            Box::new(window) as Box<dyn ::zbl::Capturable>,
            capture_cursor,
            use_staging_texture,
        )
    }

    pub fn from_display_id(
        id: usize,
        capture_cursor: bool,
        use_staging_texture: bool,
    ) -> Result<Self> {
        let display = ::zbl::Display::find_by_id(id)?;
        Self::from_capturable(
            Box::new(display) as Box<dyn ::zbl::Capturable>,
            capture_cursor,
            use_staging_texture,
        )
    }

    fn _start(&self) -> Result<()> {
        Ok(self.inner.start()?)
    }

    fn _grab(&mut self) -> Result<Option<Frame>> {
        if let Some(frame) = self.inner.grab()? {
            let desc = frame.desc();
            Ok(Some(Frame {
                width: desc.Width,
                height: desc.Height,
                row_pitch: frame.mapped_ptr.RowPitch,
                ptr: frame.mapped_ptr.pData,
            }))
        } else {
            Ok(None)
        }
    }

    fn _stop(&mut self) -> Result<()> {
        Ok(self.inner.stop()?)
    }
}

#[pymethods]
impl Capture {
    #[new]
    pub fn new(
        window_name: Option<&str>,
        window_handle: Option<i32>,
        display_id: Option<i32>,
        capture_cursor: Option<bool>,
        use_staging_texture: Option<bool>,
    ) -> PyResult<Self> {
        let capture_cursor = capture_cursor.unwrap_or(false);
        let cpu_access = use_staging_texture.unwrap_or(true);
        if let Some(name) = window_name {
            Ok(Self::from_window_name(name, capture_cursor, cpu_access)?)
        } else if let Some(handle) = window_handle {
            Ok(Self::from_capturable(
                Box::new(::zbl::Window::new(HWND(handle as isize))) as Box<dyn ::zbl::Capturable>,
                capture_cursor,
                cpu_access,
            )?)
        } else if let Some(display_id) = display_id {
            Ok(Self::from_display_id(
                display_id as usize,
                capture_cursor,
                cpu_access,
            )?)
        } else {
            Err(Error::NeitherNameNorHandleIsSet)?
        }
    }

    #[getter]
    pub fn handle(&self) -> PyResult<isize> {
        Ok(self.inner.capturable().get_raw_handle())
    }

    pub fn start(&self) -> PyResult<()> {
        Ok(self._start()?)
    }

    pub fn grab(&mut self) -> PyResult<Option<Frame>> {
        Ok(self._grab()?)
    }

    pub fn stop(&mut self) -> PyResult<()> {
        Ok(self._stop()?)
    }
}

#[pymodule]
#[pyo3(name = "zbl")]
fn zbl(_py: Python<'_>, module: &PyModule) -> PyResult<()> {
    module.add_class::<Frame>()?;
    module.add_class::<Capture>()?;
    Ok(())
}
