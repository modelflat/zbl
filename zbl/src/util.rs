use windows::{
    core::{Interface, Result},
    Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
};

pub fn get_dxgi_interface_from_object<S: Interface, R: Interface>(object: &S) -> Result<R> {
    let access: IDirect3DDxgiInterfaceAccess = object.cast()?;
    let object = unsafe { access.GetInterface::<R>()? };
    Ok(object)
}

pub fn convert_u16_string(input: &[u16]) -> String {
    let mut s = String::from_utf16_lossy(input);
    if let Some(index) = s.find('\0') {
        s.truncate(index);
    }
    s
}
