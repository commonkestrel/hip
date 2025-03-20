use tinyvec::ArrayVec;

enum JpegMarker {
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


}

impl Ssdv {
    pub fn new<C: Into<ArrayVec<[u8; 6]>>>(
        ty: PacketType,
        callsign: C,
        image_id: u8,
        mut quality: u8,
    ) -> Self {
        // limit quality to 7 at a maximum
        quality = quality.min(7);
        let call = Self::encode_callsign(callsign.into());

        todo!()
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
}

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

pub enum PacketType {
    /// Normal mode (224 byte packet + 32 byte FEC)
    Normal,
    /// No-FEC mode (256 byte packet)
    NoFEC,
}
