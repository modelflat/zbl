pub fn convert_u16_string(input: &[u16]) -> String {
    let mut s = String::from_utf16_lossy(input);
    if let Some(index) = s.find('\0') {
        s.truncate(index);
    }
    s
}
