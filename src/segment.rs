#![allow(dead_code)]
use byteorder::{LittleEndian, WriteBytesExt};

#[derive(Default, Debug)]
pub struct Segment {
    conv: u32,
    cmd: u8,
    pub frg: u8,
    wnd: u16,
    ts: u32,
    pub sn: u32,
    una: u32,
    resendts: u32,
    rto: u32,
    fastack: u32,
    xmit: u32,
    pub data: Vec<u8>,
}

impl Segment {
    pub fn new() -> Segment {
        Default::default()
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data = Vec::from(bytes);
    }

    pub fn encode(&self, buf: &mut Vec<u8>) {
        buf.write_u32::<LittleEndian>(self.conv).unwrap();
        buf.write_u8(self.cmd).unwrap();
        buf.write_u8(self.frg).unwrap();
        buf.write_u16::<LittleEndian>(self.wnd).unwrap();
        buf.write_u32::<LittleEndian>(self.ts).unwrap();
        buf.write_u32::<LittleEndian>(self.sn).unwrap();
        buf.write_u32::<LittleEndian>(self.una).unwrap();
        buf.write_u32::<LittleEndian>(self.data.len() as u32).unwrap();
    }
}

#[test]
pub fn test_segment_encode() {
    // let mut seg = Segment { data: Some(ByteBuf::mut_with_capacity(100)), ..Default::default() };
    let mut seg: Segment = Default::default();
    seg.write_bytes(&[8, 8, 8, 8]);
    seg.conv = 4;
 let mut buf = Vec::<u8>::with_capacity(100);
    seg.encode(&mut buf);
    assert!(buf ==
            &[4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0]);
}
