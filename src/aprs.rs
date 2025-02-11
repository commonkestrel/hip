pub struct Header {
    dest: Dest,
    source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Dest {
    call_sign: [u8; 6],
    ssid: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Source {
    call_sign: [u8; 6],
    ssid: u8,
}
