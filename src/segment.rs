#![allow(dead_code)]
use buf::ByteBuffer;

#[derive(Default, Debug)]
pub struct Segment {
    pub conv: u32,
    pub cmd: u32,
    pub frg: u32,
    pub wnd: u32,
    pub ts: u32,
    pub sn: u32,
    pub una: u32,
    resendts: u32,
    rto: u32,
    pub fastack: u32,
    xmit: u32,
    pub data: Vec<u8>,
}

impl Segment {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity_zeroed(cap: usize) -> Self {
        Segment { data: vec![0;cap], ..Default::default() }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data = Vec::from(bytes);
    }

    pub fn encode(&self, buf: &mut ByteBuffer) {
        buf.write_u32(self.conv);
        buf.write_u8(self.cmd as u8);
        buf.write_u8(self.frg as u8);
        buf.write_u16(self.wnd as u16);
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
    println!("{:?}", buf.to_bytes());
    assert!(buf.to_bytes() ==
            [0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]);
}
