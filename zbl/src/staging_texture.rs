use windows::{
    core::{Interface, Result},
    Win32::Graphics::{
        Direct3D11::{
            ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_BIND_FLAG,
            D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
            D3D11_RESOURCE_MISC_FLAG, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
        },
        Dxgi::Common::DXGI_FORMAT,
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
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        desc.Width = width;
        desc.Height = height;
        desc.Format = format;
        desc.MipLevels = 1;
        desc.ArraySize = 1;
        desc.SampleDesc.Count = 1;
        desc.BindFlags = D3D11_BIND_FLAG(0);
        desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;

        let texture = unsafe { device.CreateTexture2D(&desc, None)? };

        Ok(Self { texture, desc })
    }

    pub fn as_resource(&self) -> Result<ID3D11Resource> {
        self.texture.cast()
    }

    pub fn as_mapped(&self, context: &ID3D11DeviceContext) -> Result<D3D11_MAPPED_SUBRESOURCE> {
        let staging_texture_ptr: ID3D11Resource = self.texture.cast()?;
        let mapped_texture =
            unsafe { context.Map(Some(&staging_texture_ptr), 0, D3D11_MAP_READ, 0)? };
        // we can instantly unmap because the texture is staging, and will be still accessible by CPU
        // TODO there should be a way to do this by queueing a fence (we only need to wait copies) or something like that,
        // which would probably be more correct solution rather than map-unmap
        unsafe {
            context.Unmap(Some(&staging_texture_ptr), 0);
        };
        Ok(mapped_texture)
    }
}
