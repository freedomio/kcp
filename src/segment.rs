#![allow(dead_code)]
use byteorder::{LittleEndian, WriteBytesExt};
use bytes::{MutBuf, ByteBuf, MutByteBuf};

#[derive(Default, Debug)]
pub struct Segment {
    conv: u32,
    cmd: u8,
    frg: u8,
    wnd: u16,
    ts: u32,
    sn: u32,
    una: u32,
    resendts: u32,
    rto: u32,
    fastack: u32,
    xmit: u32,
    data: Option<ByteBuf>,
}

impl Segment {
	pub fn fill_data(&mut self, bytes: &[u8])  {
		self.data = Some(ByteBuf::from_slice(bytes));
	}

    pub fn encode(&self, buf: &mut MutByteBuf) {
        buf.write_u32::<LittleEndian>(self.conv).unwrap();
        buf.write_u8(self.cmd).unwrap();
        buf.write_u8(self.frg).unwrap();
        buf.write_u16::<LittleEndian>(self.wnd).unwrap();
        buf.write_u32::<LittleEndian>(self.ts).unwrap();
        buf.write_u32::<LittleEndian>(self.sn).unwrap();
        buf.write_u32::<LittleEndian>(self.una).unwrap();
        let len = match self.data {
            Some(ref b) => b.capacity(),
            None => 0,
        };
        buf.write_u32::<LittleEndian>(len as u32).unwrap();
    }
}

#[test]
pub fn test_segment_encode() {
    // let mut seg = Segment { data: Some(ByteBuf::mut_with_capacity(100)), ..Default::default() };
	let mut seg: Segment = Default::default();
	seg.fill_data(&[8,8,8,8]);
    seg.conv = 4;
    let mut buf = ByteBuf::mut_with_capacity(100);
    seg.encode(&mut buf);
	assert!(buf.bytes() == &[4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0]);
}
