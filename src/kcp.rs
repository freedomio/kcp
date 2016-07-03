#![allow(dead_code)]
use byteorder::{LittleEndian, WriteBytesExt};
use bytes::{MutBuf, ByteBuf, MutByteBuf};
use segment::Segment;
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


enum Command {
    /// Push send the packet
    Push,
    /// Ack the packet
    Ack,
    /// Wask the cmd is ask about the other side to get the window's size
    Wask,
    /// Wins tell the other side the size of window
    Wins,
}


#[derive(Default)]
struct KCP<'k> {
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

    snd_queue: &'k [Segment],
    rcv_queue: &'k [Segment],
    snd_buf: &'k [Segment],
    rcv_buf: &'k [Segment],

    acklist: &'k [u32],
    buffer: Option<MutByteBuf>,
    fastresend: i32,
    nocwnd: i32,
    logmask: i32,
    output: Option<fn(buf: &mut [u8], size: i32)>,
}



fn new_kcp<'k>(conv: u32, output: fn(buf: &mut [u8], size: i32)) -> KCP<'k> {
    let mut kcp = KCP { ..Default::default() };
    kcp.conv = conv;
    kcp.snd_wnd = WND_SND;
    kcp.rcv_wnd = WND_RCV;
    kcp.rmt_wnd = WND_RCV;
    kcp.mtu = MTU_DEF;
    kcp.mss = kcp.mtu - OVERHEAD;
    kcp.buffer = Some(ByteBuf::mut_with_capacity(100));
    kcp.rx_rto = RTO_DEF;
    kcp.rx_minrto = RTO_MIN;
    kcp.interval = INTERVAL;
    kcp.ts_flush = INTERVAL;
    kcp.ssthresh = THRESH_INIT;
    kcp.dead_link = DEADLINK;
    kcp.output = Some(output);
    return kcp;
}

impl<'k> KCP<'k> {
    fn peek_size(&self) -> i32 {
        if self.rcv_queue.len() == 0 {
            return -1;
        }

        let seg = &self.rcv_queue[0];
        if seg.frg == 0 {
            match seg.data {
                Some(ref data) => return data.capacity() as i32,
                None => return -1,
            }
        }

        if self.rcv_queue.len() < (seg.frg as usize) {
            return -1;
        }

        let mut length: i32 = 0;
        for segment in self.rcv_queue {
            match segment.data {
                Some(ref data) => length += data.capacity() as i32,
                None => length += 0,
            }
            // length += segment.dalen();
            if seg.frg == 0 {
                break;
            }
        }
        return length;
    }
    fn recv(&self, buffer: ByteBuf) -> i32 {
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
        let fast_recover: bool;
        if self.rcv_queue.len() >= self.rcv_wnd as usize {
            fast_recover = true;
        }
        let count = 0;
        for seg in self.rcv_queue {
            match seg.data {
                Some(ref data) => buffer = data,
                None => return -1,
            }

        }



        return -1;
    }
}

fn output(buf: &mut [u8], size: i32) {
    println!("this is output test fn");
}
#[test]
fn newKCP_test() {
    let kcp = new_kcp(22, output);
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
