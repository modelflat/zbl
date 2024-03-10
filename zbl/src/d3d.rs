use windows::{
    core::{Interface, Result},
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
            },
            Dxgi::{IDXGIDevice, DXGI_ERROR_UNSUPPORTED},
        },
        System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
    },
};

fn create_d3d_device() -> Result<ID3D11Device> {
    for driver_type in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device = None;
        let result = unsafe {
            D3D11CreateDevice(
                None,
                driver_type,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
        match result {
            Ok(_) => return Ok(device.unwrap()),
            Err(e) if e.code() == DXGI_ERROR_UNSUPPORTED => continue,
            Err(e) => return Err(e),
        };
    }

    // TODO result
    panic!("failed to create D3D device with any of the supported driver types");
}

fn create_direct3d_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice> {
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
    inspectable.cast()
}

pub struct D3D {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub direct3d_device: IDirect3DDevice,
}

impl D3D {
    pub fn new() -> Result<Self> {
        let device = create_d3d_device()?;
        let context = unsafe { device.GetImmediateContext()? };
        let direct3d_device = create_direct3d_device(&device)?;
        Ok(Self {
            device,
            context,
            direct3d_device,
        })
    }
}
