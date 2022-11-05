use windows::{
    core::{Abi, Interface, Result},
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_SDK_VERSION,
            },
            Dxgi::{IDXGIDevice, DXGI_ERROR_UNSUPPORTED},
        },
        System::WinRT::Direct3D11::{
            CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
        },
    },
};

pub fn create_d3d_device() -> Result<ID3D11Device> {
    for driver_type in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device = None;
        let result = unsafe {
            D3D11CreateDevice(
                None,
                driver_type,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION as u32,
                Some(&mut device as *mut _),
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
    panic!("failed to create D3D device with any of the types");
}

pub fn create_direct3d_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice> {
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
    inspectable.cast()
}

pub fn get_dxgi_interface_from_object<S: Interface, R: Interface + Abi>(object: &S) -> Result<R> {
    let access: IDirect3DDxgiInterfaceAccess = object.cast()?;
    let object = unsafe { access.GetInterface::<R>()? };
    Ok(object)
}
