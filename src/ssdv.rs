use tinyvec::ArrayVec;

pub type Result<T> = std::result::Result<T, SsdvError>;

/// Maximum size for the DQT and DHT tables
const TABLE_LEN: usize = 546;

/// Extra space for reading marker data
const HBUF_LEN: usize = 16;

pub enum SsdvError {
    /// Not enough memory available (shouldn't happen!)
    Memory,
    /// Progressive images are not supported
    Progressive
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
enum JpegMarker {
    Invalid = 0x0000,
    Tem = 0xFF01,
    Sof0 = 0xFFC0,
    Sof1,
    Sof2,
    Sof3,
    Dht,
    Sof5,
    Sof6,
    Sof7,
    Jpg,
    Sof9,
    Sof10,
    Sof11,
    Dac,
    Sof13,
    Sof14,
    Sof15,
    Rst0,
    Rst1,
    Rst2,
    Rst3,
    Rst4,
    Rst5,
    Rst6,
    Rst7,
    Soi,
    Eoi,
    Sos,
    Dqt,
    Dnl,
    Dri,
    DHP,
    Exp,
    App0,
    App1,
    App2,
    App3,
    App4,
    App5,
    App6,
    App7,
    App8,
    App9,
    App10,
    App11,
    App12,
    App13,
    App14,
    App15,
    Jpg0,
    Jpg1,
    Jpg2,
    Jpg3,
    Jpg4,
    Jpg5,
    Jpg6,
    Sof48,
    Lse,
    Jpg9,
    Jpg10,
    Jpg11,
    Jpg12,
    Jpg13,
    Com,
}

impl PartialEq<u16> for JpegMarker {
    fn eq(&self, other: &u16) -> bool {
        return *self as u16 == *other
    }
}

impl PartialEq<JpegMarker> for u16 {
    fn eq(&self, other: &JpegMarker) -> bool {
        return *self == *other as u16;
    }
}

impl PartialOrd<u16> for JpegMarker {
    fn partial_cmp(&self, other: &u16) -> Option<std::cmp::Ordering> {
        return (*self as u16).partial_cmp(other);
    }
}

impl PartialOrd<JpegMarker> for u16 {
    fn partial_cmp(&self, other: &JpegMarker) -> Option<std::cmp::Ordering> {
        return self.partial_cmp(&(*other as u16));
    }
}

impl From<u16> for JpegMarker {
    fn from(value: u16) -> Self {
        if value != JpegMarker::Tem && (value < JpegMarker::Sof0 || value > JpegMarker::Com) {
            return JpegMarker::Invalid;
        }

        return unsafe { std::mem::transmute(value) };
    }
} 

/// APP0 header data
const APP0: [u8; 14] = [
    0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00,
];

/// SOS header data
const SOS: [u8; 10] = [0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00];

/// Quantisation table scaling factors for each quality level 0-7
const DQT_SCALES: [u16; 8] = [5000, 357, 172, 116, 100, 58, 28, 0];

pub struct Ssdv {
    ty: u8,
    callsign: u32,

    payload_size: u16,
    crc_data_size: u16,

    width: u16,
    height: u16,
    image_id: u8,
    packet_id: u16,
    mcu_mode: u8,
    mcu_id: u16,
    mcu_count: u16,
    quality: u8,
    packet_mcu_id: u16,
    packet_mcu_offset: u16,

    /// 
    buf: Box<dyn Iterator<Item = u8>>,
    skip: usize,

    /// Input bits currently being worked on
    workbits: u32,
    /// Number of bits in the input bit buffer
    worklen: u8,

    outbits: u32,
    outlen: u8,

    state: State,
    marker: u16,
    marker_len: u16,
    marker_data: Vec<u8>,
    component: u8,
    ycparts: u8,
    mcupart: u8,
    acpart: u8,
    dc: [isize; 3],
    adc: [isize; 3],
    acrle: u8,
    accrle: u8,
    dri: u16,
    decoding: bool,
    reset_mcu: u32,
    needbits: u8,

    // The input huffman and quantisation tables
    stbls: [u8; TABLE_LEN + HBUF_LEN],
    sdht: [[u8; 2]; 2],
    sdqt: [u8; 2],
    stbl_len: usize,

    dtbls: [u8; TABLE_LEN],
    ddht: [[u8; 2]; 2],
    ddqt: [u8; 2],
    dtbl_len: usize,
}

impl Ssdv {
    pub fn new<C: Into<ArrayVec<[u8; 6]>>, I: IntoIterator<Item = u8>>(
        ty: PacketType,
        callsign: C,
        image_id: u8,
        mut quality: u8,
        image: I
    ) -> Self {
        // limit quality to 7 at a maximum
        quality = quality.min(7);
        let call = Self::encode_callsign(callsign.into());

        let buf = Box::new(image.into_iter());

        todo!()

        // Ssdv {
        //     ty,
        //     callsign: call,

        // }
    }

    fn encode_callsign(callsign: ArrayVec<[u8; 6]>) -> u32 {
        let mut x: u32 = 0;

        for c in callsign.into_iter().rev() {
            x *= 40;
            if c >= b'A' && c <= b'Z' {
                x += (c - b'A' + 14) as u32;
            } else if c >= b'a' && c <= b'z' {
                x += (c - b'a' + 14) as u32;
            } else if c >= b'0' && c <= b'9' {
                x += (c - b'0' + 1) as u32;
            }
        }

        return x;
    }

    // fn outbits(&mut self, bits: u16, length: u8) {
    //     if length > 0 {
    //         self.outbits <<= length;
    //         self.outbits |= (bits & ((1 << length) - 1)) as u32;
    //         self.outlen += length;
    //     }

    //     while self.outlen >= 8 && self.outlen
    // }

    fn have_marker(&mut self) -> Result<()> {
        use JpegMarker as JM;

        match self.marker.into() {
            JM::Sof0 | JM::Sos | JM::Dri | JM::Dht | JM::Dqt => {
                if self.marker_len as usize > TABLE_LEN + HBUF_LEN - self.stbl_len {
                    return Err(SsdvError::Memory);
                }

                self.marker_data = Vec::new();
            }
            _ => {
                self.skip = self.marker_len as usize;
                self.state = State::Marker;
            }
        }

        Ok(())
    }
}

impl Iterator for Ssdv {
    type Item = Result<[u8; 256]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == State::Eoi {
            return None;
        }

        let mut out: ArrayVec<[u8; 256]> = ArrayVec::new();

        while let Some(b) = self.buf.next() {
            if self.skip > 0 {
                self.skip -= 1;
                continue;
            }

            match self.state {
                State::Marker => {
                    self.marker = (self.marker << 8) | b as u16;

                    if self.marker == JpegMarker::Tem || (self.marker >= JpegMarker::Rst0 && self.marker <= JpegMarker::Com) {
                        self.marker_len = 0;
                        if let Err(err) = self.have_marker() {
                            return Some(Err(err))
                        }
                    }
                }
            }
        }

        return Some(Ok(out.into_inner()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SsdvPacketInfo {
    ty: u8,
    callsign: ArrayVec<[u8; 6]>,
    image_id: u8,
    packet_id: u16,
    width: u16,
    height: u16,
    eoi: u8,
    quality: u8,
    mcu_mode: u16,
    mcu_offset: u8,
    mcu_id: u16,
    mcu_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PacketType {
    /// Normal mode (224 byte packet + 32 byte FEC)
    Normal,
    /// No-FEC mode (256 byte packet)
    NoFEC,
    Padding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum State {
    Marker,
    MarkerLen,
    MarkerData,
    Huff,
    Int,
    Eoi,
}
