use fixbuf::ByteBuffer;

#[derive(Default, Debug)]
pub struct Segment {
    pub conv: u32,
    pub cmd: u32,
    pub frg: u32,
    pub wnd: u32,
    pub ts: u32,
    pub sn: u32,
    pub una: u32,
    pub resendts: u32,
    pub rto: u32,
    pub fastack: u32,
    pub xmit: u32,
    pub data: Vec<u8>,
}

impl Segment {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity_zeroed(cap: usize) -> Self {
        Segment { data: vec![0;cap], ..Default::default() }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut seg = Segment::new();
        seg.data = Vec::from(bytes);
        seg
    }

    pub fn encode(&self, buf: &mut ByteBuffer) {
        buf.write_u32(self.conv).unwrap();
        buf.write_u8(self.cmd as u8).unwrap();
        buf.write_u8(self.frg as u8).unwrap();
        buf.write_u16(self.wnd as u16).unwrap();
        buf.write_u32(self.ts).unwrap();
        buf.write_u32(self.sn).unwrap();
        buf.write_u32(self.una).unwrap();
        buf.write_u32(self.data.len() as u32).unwrap();
    }

    pub fn data_bytes(&self) -> Vec<u8> {
        self.data.to_vec()
    }
}
