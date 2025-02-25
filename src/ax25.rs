#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
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

impl Into<[u8; 16]> for Header {
    fn into(self) -> [u8; 16] {
        let dcs = self.dest.call_sign;
        let scs = self.source.call_sign;

        [
            dcs[0],
            dcs[1],
            dcs[2],
            dcs[3],
            dcs[4],
            dcs[5],
            self.dest.ssid,
            scs[0],
            scs[1],
            scs[2],
            scs[3],
            scs[4],
            scs[5],
            self.source.ssid,
            0x03,
            0xf0,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coordinate {
    degree: i16,
    minute: u8,
    second: u8,
}
