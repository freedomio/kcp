#![allow(dead_code)]

use segment::Segment;
use std::collections::VecDeque;
use buf::ByteBuffer;
/// all time value is milliseconds
/// retransmission timeout with no delay but at least 30 ms
const RTO_NDL: u32 = 30;
/// the min value of retransmission timeout
const RTO_MIN: u32 = 100;
/// the normal value of retransmission timeout
const RTO_DEF: u32 = 200;
/// the max value of retransmission timeout
const RTO_MAX: u32 = 60000;

/// for the cmd Wask
const ASK_SEND: u32 = 1;
/// for the cmd Wins
const ASK_TELL: u32 = 2;

/// the size of window for send
const WND_SND: u32 = 32;
/// the size of window for receive
const WND_RCV: u32 = 32;

/// the default MTU(Maxitum Transmission Unit) value
const MTU_DEF: u32 = 1400;

const INTERVAL: u32 = 100;
/// the size of headers
const OVERHEAD: u32 = 24;

const DEADLINK: u32 = 20;
///  the initialization of ssthresh(Slow-Start Threshold)
const THRESH_INIT: u32 = 2;
///  the min of ssthresh
const THRESH_MIN: u32 = 2;
/// the time to wait the probe window size
const PROBE_INIT: u32 = 7000;
const PROBE_LIMIT: u32 = 120000;


/// Push send the packet
const CMD_PUSH: u32 = 81;
/// Ack the packet
const CMD_ACK: u32 = 82;
/// Wask the cmd is ask about the other side to get the window's size
const CMD_WASK: u32 = 83;
/// Wins tell the other side the size of window
const CMD_WINS: u32 = 84;


#[derive(Default)]
struct KCP {
    conv: u32,
    mtu: u32,
    mss: u32,
    state: u32,
    snd_una: u32,
    snd_nxt: u32,
    rcv_nxt: u32,
    ts_recent: u32,
    ts_lastack: u32,
    ssthresh: u32,
    rx_rttval: u32,
    rx_srtt: u32,
    rx_rto: u32,
    rx_minrto: u32,
    snd_wnd: u32,
    rcv_wnd: u32,
    rmt_wnd: u32,
    cwnd: u32,
    probe: u32,
    current: u32,
    interval: u32,
    ts_flush: u32,
    xmit: u32,
    nodelay: u32,
    updated: u32,
    ts_probe: u32,
    probe_wait: u32,
    dead_link: u32,
    incr: u32,

    snd_queue: VecDeque<Segment>,
    rcv_queue: VecDeque<Segment>,
    snd_buf: VecDeque<Segment>,
    rcv_buf: VecDeque<Segment>,

    acklist: Vec<u32>,
    buffer: Option<ByteBuffer>,
    fastresend: i32,
    nocwnd: i32,
    logmask: i32,
    output: Option<fn(buf: &mut ByteBuffer)>,
}

impl KCP {
    fn new(conv: u32, output: fn(buf: &mut ByteBuffer)) -> Self {
        let mut kcp = KCP { ..Default::default() };
        kcp.conv = conv;
        kcp.snd_wnd = WND_SND;
        kcp.rcv_wnd = WND_RCV;
        kcp.rmt_wnd = WND_RCV;
        kcp.mtu = MTU_DEF;
        kcp.mss = kcp.mtu - OVERHEAD;
        kcp.rx_rto = RTO_DEF;
        kcp.rx_minrto = RTO_MIN;
        kcp.interval = INTERVAL;
        kcp.ts_flush = INTERVAL;
        kcp.ssthresh = THRESH_INIT;
        kcp.dead_link = DEADLINK;
        kcp.buffer = Some(ByteBuffer::new());
        kcp.output = Some(output);
        return kcp;
    }

    fn peek_size(&self) -> i32 {
        if self.rcv_queue.len() == 0 {
            return -1;
        }

        let seg = &self.rcv_queue[0];
        if seg.frg == 0 {
            return seg.data.len() as i32;
        }

        if self.rcv_queue.len() < (seg.frg as usize) {
            return -1;
        }

        let mut length: i32 = 0;
        for seg in &self.rcv_queue {
            length += seg.data.len() as i32;
            if seg.frg == 0 {
                break;
            }
        }
        return length;
    }

    fn recv(&mut self, buffer: &mut Vec<u8>) -> i32 {
        if self.rcv_queue.len() == 0 {
            return -1;
        }
        let size = self.peek_size();
        if size < 0 {
            return -2;
        }
        if size > buffer.capacity() as i32 {
            return -3;
        }
        let mut fast_recover = false;
        if self.rcv_queue.len() >= self.rcv_wnd as usize {
            fast_recover = true;
        }
        let mut num = 0;
        loop {
            match self.rcv_queue.pop_front() {
                Some(ref mut seg) => {
                    buffer.append(&mut seg.data);
                    num += seg.data.len() as i32;
                    if seg.frg == 0 {
                        break;
                    }
                }
                None => break,
            }

        }
        loop {
            match self.rcv_buf.pop_front() {
                Some(seg) => {
                    if seg.sn == self.rcv_nxt && self.rcv_queue.len() < self.rcv_wnd as usize {
                        self.rcv_queue.push_back(seg);
                        self.rcv_nxt += 1;
                    }
                }
                None => break,
            }
        }
        if self.rcv_queue.len() < self.rcv_wnd as usize && fast_recover {
            self.probe |= ASK_TELL;
        }
        return num;
    }

    fn ack_push(&mut self, sn: u32, ts: u32) {
        self.acklist.push(sn);
        self.acklist.push(ts);
    }

    fn ack_get(&self, p: i32) -> (u32, u32) {
        (self.acklist[(p * 2) as usize], self.acklist[(p * 2 + 2) as usize])
    }

    fn parse_data(&mut self, new_seg: Segment) {
        let sn = new_seg.sn;
        if sn >= (self.rcv_nxt + self.rcv_wnd) || sn < self.rcv_nxt {
            // TODO: need process
            return;
        }
        for i in (0..self.rcv_buf.len()).rev() {
            if self.rcv_buf[i].sn == sn {
                break;
            }
            if sn - self.rcv_buf[i].sn > 0 {
                self.rcv_buf.insert(i + 1, new_seg);
                break;
            }
        }
        loop {
            match self.rcv_buf.pop_front() {
                Some(seg) => {
                    if seg.sn == self.rcv_nxt && (self.rcv_queue.len() as u32) < self.rcv_wnd {
                        self.rcv_queue.push_back(seg);
                        self.rcv_nxt += 1;
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
    }

    fn parse_ack(&mut self, sn: u32) {
        if sn < self.snd_una || sn >= self.snd_nxt {
            return;
        }
        for i in 0..self.snd_buf.len() {
            if sn == self.snd_buf[i].sn {
                self.snd_buf.remove(i);
                break;
            }
            if sn < self.snd_buf[i].sn {
                break;
            }
        }
    }

    fn parse_fastack(&mut self, sn: u32) {
        if sn < self.snd_una || sn >= self.snd_nxt {
            return;
        }
        for seg in &mut self.snd_buf {
            if sn != seg.sn {
                seg.fastack += 1;
            } else {
                break;
            }
        }
    }

    fn parse_una(&mut self, una: u32) {
        for i in 0..self.snd_buf.len() {
            if una > self.snd_buf[i].sn {
                self.snd_buf.remove(i);
                break;
            }
            break;
        }
    }

    fn wnd_unused(&mut self) -> u32 {
        if self.rcv_queue.len() < self.rcv_wnd as usize {
            return self.rcv_wnd - self.rcv_queue.len() as u32;
        }
        0
    }

    fn flush(&mut self) {
        if self.updated == 0 {
            return;
        }
        let (current, change, lost) = (self.current, 0, false);
        let mut seg = Segment::new();
        seg.conv = self.conv;
        seg.cmd = CMD_ACK;
        seg.wnd = self.wnd_unused();
        seg.una = self.rcv_nxt;
        if let Some(ref mut buffer) = self.buffer {
            for _ in 0..self.acklist.len() / 2 {
                {
                    if let Some(output) = self.output {
                        output(buffer);
                    }
                }
            }
        }

    }
}

fn output(buf: &mut ByteBuffer) {
    println!("this is output test fn");
    println!("buf: {:?}, size: {:?}", buf.to_bytes(), buf.get_rpos());
}

#[test]
fn test_kcp() {
    let kcp = KCP::new(22, output);
    assert!(kcp.conv == 22);
    assert!(kcp.snd_wnd == WND_SND);
    assert!(kcp.rcv_wnd == WND_RCV);
    assert!(kcp.rmt_wnd == WND_RCV);
    assert!(kcp.mtu == MTU_DEF);
    assert!(kcp.mss == kcp.mtu - OVERHEAD);
    // kcp.buffer = Some(ByteBuf::mut_with_capacity(100));
    assert!(kcp.rx_rto == RTO_DEF);
    assert!(kcp.rx_minrto == RTO_MIN);
    assert!(kcp.interval == INTERVAL);
    assert!(kcp.ts_flush == INTERVAL);
    assert!(kcp.ssthresh == THRESH_INIT);
    assert!(kcp.dead_link == DEADLINK);
}
