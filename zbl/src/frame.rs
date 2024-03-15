use windows::Win32::Graphics::Direct3D11::{
    ID3D11Texture2D, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
};

pub struct Frame {
    pub texture: ID3D11Texture2D,
    pub mapped_ptr: D3D11_MAPPED_SUBRESOURCE,
}

impl Frame {
    pub fn new(texture: ID3D11Texture2D, mapped_ptr: D3D11_MAPPED_SUBRESOURCE) -> Self {
        Self {
            texture,
            mapped_ptr,
        }
    }

    pub fn desc(&self) -> D3D11_TEXTURE2D_DESC {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { self.texture.GetDesc(&mut desc) };
        desc
    }
}
