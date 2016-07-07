#![allow(dead_code)]
use byteorder::{ByteOrder, LittleEndian};
use std::io::Write;

#[derive(Clone,Copy,Debug)]
pub enum BufferResult {
    ReadOverflow,
    WriteOverflow,
}

#[derive(Default)]
pub struct ByteBuffer {
    data: Vec<u8>,
    wpos: usize,
    rpos: usize,
}

impl ByteBuffer {
    pub fn with_capacity(cap: usize) -> Self {
        ByteBuffer {
            data: vec![0; cap],
            wpos: 0,
            rpos: 0,
        }
    }

    /// Return the buffer size
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn read_remain(&self) -> usize {
        self.len() - self.rpos
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), BufferResult> {
        let size = bytes.len() + self.wpos;
        if size > self.data.len() {
            return Err(BufferResult::WriteOverflow);
        }
        for v in bytes {
            self.data[self.wpos] = *v;
            self.wpos += 1;
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.wpos = 0;
        self.rpos = 0;
    }

    /// Append a byte (8 bits value) to the buffer
    pub fn write_u8(&mut self, val: u8) -> Result<(), BufferResult> {
        self.write_bytes(&[val])
    }

    /// Append a word (16 bits value) to the buffer
    pub fn write_u16(&mut self, val: u16) -> Result<(), BufferResult> {
        let mut buf = [0; 2];
        LittleEndian::write_u16(&mut buf, val);
        self.write_bytes(&buf)
    }

    /// Append a double word (32 bits value) to the buffer
    pub fn write_u32(&mut self, val: u32) -> Result<(), BufferResult> {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, val);
        self.write_bytes(&buf)
    }


    /// Read a defined amount of raw bytes. The program crash if not enough bytes are available
    pub fn read_bytes(&mut self, size: usize) -> Result<Vec<u8>, BufferResult> {
        if self.rpos + size > self.data.len() {
            return Err(BufferResult::ReadOverflow);
        }
        let range = self.rpos..self.rpos + size;
        let mut res = Vec::<u8>::new();
        res.write(&self.data[range]).unwrap();
        self.rpos += size;
        Ok(res)
    }

    /// Read one byte. The program crash if not enough bytes are available
    pub fn read_u8_as_u32(&mut self) -> Result<u32, BufferResult> {
        if self.rpos >= self.data.len() {
            return Err(BufferResult::ReadOverflow);
        }
        let pos = self.rpos;
        self.rpos += 1;
        Ok(self.data[pos] as u32)
    }

    /// Read a 2-bytes long value. The program crash if not enough bytes are available
    pub fn read_u16_as_u32(&mut self) -> Result<u32, BufferResult> {
        if self.rpos + 2 >= self.data.len() {
            return Err(BufferResult::ReadOverflow);
        }
        let range = self.rpos..self.rpos + 2;
        self.rpos += 2;
        Ok(LittleEndian::read_u16(&self.data[range]) as u32)
    }

    /// Read a four-bytes long value. The program crash if not enough bytes are available
    pub fn read_u32(&mut self) -> Result<u32, BufferResult> {
        if self.rpos + 4 >= self.data.len() {
            return Err(BufferResult::ReadOverflow);
        }
        let range = self.rpos..self.rpos + 4;
        self.rpos += 4;
        Ok(LittleEndian::read_u32(&self.data[range]))
    }

    /// Return the position of the reading cursor
    pub fn get_rpos(&self) -> usize {
        self.rpos
    }

    /// Return the writing cursor position
    pub fn get_wpos(&self) -> usize {
        self.wpos
    }

    /// Return the raw byte buffer.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.to_vec()
    }
}
