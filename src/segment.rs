#![allow(dead_code)]
use bytebuffer::ByteBuffer;

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
    pub fn new() -> Self {
        Default::default()
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data = Vec::from(bytes);
    }

    pub fn encode(&self, buf: &mut ByteBuffer) {
        buf.write_u32(self.conv);
        buf.write_u8(self.cmd);
        buf.write_u8(self.frg);
        buf.write_u16(self.wnd);
        buf.write_u32(self.ts);
        buf.write_u32(self.sn);
        buf.write_u32(self.una);
        buf.write_u32(self.data.len() as u32);
    }
}

#[test]
pub fn test_segment_encode() {
    // let mut seg = Segment { data: Some(ByteBuf::mut_with_capacity(100)), ..Default::default() };
    let mut seg: Segment = Default::default();
    seg.write_bytes(&[8, 8, 8, 8]);
    seg.conv = 4;
	let mut buf = ByteBuffer::new();
    seg.encode(&mut buf);
    assert!(buf.to_bytes() == [0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]);
}
