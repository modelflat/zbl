use windows::{
    core::{Interface, Result},
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource,
                ID3D11Texture2D, D3D11_BOX, D3D11_CPU_ACCESS_FLAG, D3D11_CPU_ACCESS_READ,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
                D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
            },
            Dxgi::{
                Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC},
                IDXGIDevice, DXGI_ERROR_UNSUPPORTED,
            },
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

    /**
     * Create a new D3D11 Texture.
     */
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        format: DXGI_FORMAT,
        cpu_access: bool,
    ) -> Result<ID3D11Texture2D> {
        let usage = if cpu_access {
            D3D11_USAGE_STAGING
        } else {
            D3D11_USAGE_DEFAULT
        };

        let cpu_access_flags = if cpu_access {
            D3D11_CPU_ACCESS_READ
        } else {
            D3D11_CPU_ACCESS_FLAG(0)
        };

        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            Format: format,
            MipLevels: 1,
            ArraySize: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BindFlags: 0,
            MiscFlags: 0,
            Usage: usage,
            CPUAccessFlags: cpu_access_flags.0 as u32,
        };

        let mut texture = None;
        unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11device-createtexture2d
            self.device
                .CreateTexture2D(&desc, None, Some(&mut texture))?;
        }

        Ok(texture.expect("CreateTexture2D returned nullptr instead of texture"))
    }

    /**
     * Map-Unmap the texture.
     */
    pub fn map_unmap_texture(&self, texture: &ID3D11Texture2D) -> Result<D3D11_MAPPED_SUBRESOURCE> {
        let staging_texture_ptr: ID3D11Resource = texture.cast()?;
        let mut mapped_texture = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-map
            self.context.Map(
                Some(&staging_texture_ptr),
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped_texture),
            )?;
        }
        // we can instantly unmap because the texture is staging, and will be still accessible by CPU
        // TODO there should be a way to do this by queueing a fence (we only need to wait copies) or something like that,
        // which would probably be a more correct solution rather than map-unmap
        unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-unmap
            self.context.Unmap(Some(&staging_texture_ptr), 0);
        }
        Ok(mapped_texture)
    }

    /**
     * Copy the texture from src to dst.
     */
    pub fn copy_texture(
        &self,
        src: &ID3D11Texture2D,
        dst: &ID3D11Texture2D,
        region: &D3D11_BOX,
    ) -> Result<()> {
        unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-copysubresourceregion
            self.context.CopySubresourceRegion(
                Some(&dst.cast()?),
                0,
                0,
                0,
                0,
                Some(&src.cast()?),
                0,
                Some(region),
            );
        }
        Ok(())
    }
}
