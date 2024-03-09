use windows::{
    core::{Interface, Result},
    Win32::Graphics::{
        Direct3D11::{
            ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
            D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
            D3D11_USAGE_STAGING,
        },
        Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC},
    },
};

#[derive(Clone, Debug)]
pub struct StagingTexture {
    pub texture: ID3D11Texture2D,
    pub desc: D3D11_TEXTURE2D_DESC,
}

impl StagingTexture {
    pub fn new(
        device: &ID3D11Device,
        width: u32,
        height: u32,
        format: DXGI_FORMAT,
    ) -> Result<Self> {
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
            Usage: D3D11_USAGE_STAGING,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        };

        let mut texture = None;
        unsafe {
            device.CreateTexture2D(&desc, None, Some(&mut texture))?;
        }

        Ok(Self {
            texture: texture.expect("CreateTexture2D"),
            desc,
        })
    }

    pub fn as_resource(&self) -> Result<ID3D11Resource> {
        self.texture.cast()
    }

    pub fn as_mapped(&self, context: &ID3D11DeviceContext) -> Result<D3D11_MAPPED_SUBRESOURCE> {
        let staging_texture_ptr: ID3D11Resource = self.texture.cast()?;
        let mut mapped_texture = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            context.Map(
                Some(&staging_texture_ptr),
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped_texture),
            )?;
        }
        // we can instantly unmap because the texture is staging, and will be still accessible by CPU
        // TODO there should be a way to do this by queueing a fence (we only need to wait copies) or something like that,
        // which would probably be more correct solution rather than map-unmap
        unsafe {
            context.Unmap(Some(&staging_texture_ptr), 0);
        };
        Ok(mapped_texture)
    }
}
