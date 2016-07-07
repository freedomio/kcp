#![allow(dead_code)]
#![allow(unused_assignments)]

use segment::Segment;
use std::collections::VecDeque;
use buf::ByteBuffer;
use std::{i32, u32};
use std::cmp::{min, max};
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
pub struct KCP {
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
    buffer: ByteBuffer,
    fastresend: i32,
    nocwnd: i32,
    logmask: i32,
    output: Option<fn(buf: &mut ByteBuffer, size: usize)>,
}

impl KCP {
    pub fn new(conv: u32, output: fn(buf: &mut ByteBuffer, size: usize)) -> Self {
        let mut kcp = KCP { ..Default::default() };
        kcp.conv = conv;
        kcp.snd_wnd = WND_SND;
        kcp.rcv_wnd = WND_RCV;
        kcp.rmt_wnd = WND_RCV;
        kcp.mtu = MTU_DEF;
        kcp.mss = MTU_DEF - OVERHEAD;
        kcp.rx_rto = RTO_DEF;
        kcp.rx_minrto = RTO_MIN;
        kcp.interval = INTERVAL;
        kcp.ts_flush = INTERVAL;
        kcp.ssthresh = THRESH_INIT;
        kcp.dead_link = DEADLINK;
        kcp.buffer = ByteBuffer::with_capacity(((MTU_DEF + OVERHEAD) * 3) as usize);
        kcp.output = Some(output);
        return kcp;
    }

    pub fn peek_size(&self) -> isize {
        if let Some(seg) = self.rcv_queue.get(0) {
            if seg.frg == 0 {
                return seg.data.len() as isize;
            }
            if self.rcv_queue.len() < ((seg.frg + 1) as usize) {
                return -1;
            }
        } else {
            return -1;
        }
        let mut length: usize = 0;
        for seg in &self.rcv_queue {
            length += seg.data.len();
            if seg.frg == 0 {
                break;
            }
        }
        length as isize
    }

    pub fn recv(&mut self, buffer: &mut ByteBuffer) -> isize {
        if self.rcv_queue.is_empty() {
            return -1;
        }
        let peeksize = self.peek_size();
        if peeksize < 0 {
            return -2;
        }
        if peeksize as usize > buffer.len() {
            return -3;
        }
        let mut fast_recover = false;
        if self.rcv_queue.len() >= self.rcv_wnd as usize {
            fast_recover = true;
        }
        let mut num: usize = 0;
        loop {
            match self.rcv_queue.pop_front() {
                Some(ref mut seg) => {
                    buffer.write_bytes(&seg.data).unwrap();
                    num += seg.data.len();
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
        return num as isize;
    }

    pub fn send(&mut self, buffer: &mut ByteBuffer) -> isize {
        if buffer.len() == 0 {
            return -1;
        }
        let mut count: usize = if buffer.len() < self.mss as usize {
            1
        } else {
            (buffer.len() + (self.mss as usize) - 1) / (self.mss as usize)
        };
        if count > 255 {
            return -2;
        }
        if count == 0 {
            count = 1;
        }
        for i in 0..count {
            let size = min(buffer.read_remain(), self.mss as usize);
            let mut seg = Segment::from_bytes(&buffer.read_bytes(size).unwrap());
            seg.frg = (count - i - 1) as u32;
            self.snd_queue.push_back(seg);
        }
        return 0;
    }

    /// when you received a low level packet (eg. UDP packet), call it
    pub fn input(&mut self, data: &mut ByteBuffer) -> isize {
        let una = self.snd_una;
        if data.len() < OVERHEAD as usize {
            return -1;
        }
        let mut maxack: u32 = 0;
        let mut flag: isize = 0;

        loop {
            if data.len() < OVERHEAD as usize {
                break;
            }
            let conv = data.read_u32().unwrap();
            if conv != self.conv {
                return -1;
            }
            let cmd = data.read_u8_as_u32().unwrap();
            let frg = data.read_u8_as_u32().unwrap();
            let wnd = data.read_u16_as_u32().unwrap();
            let ts = data.read_u32().unwrap();
            let sn = data.read_u32().unwrap();
            let una = data.read_u32().unwrap();
            let length = data.read_u32().unwrap();
            if data.read_remain() < length as usize {
                return -2;
            }
            if cmd != CMD_PUSH && cmd != CMD_ACK && cmd != CMD_WASK && cmd != CMD_WINS {
                return -3;
            }
            self.rmt_wnd = wnd;
            self.parse_una(una);
            self.shrink_buf();

            if cmd == CMD_ACK {
                if self.current >= ts {
                    let rtt = sub_u32(self.current, ts) as u32;
                    self.update_ack(rtt);
                }
                self.parse_ack(sn);
                self.shrink_buf();
                if flag == 0 {
                    flag = 1;
                    maxack = sn;
                } else if sn > maxack {
                    maxack = sn;
                }
            } else if cmd == CMD_PUSH {
                if sn < (self.rcv_nxt + self.rcv_wnd) {
                    self.ack_push(sn, ts);
                    if sn >= self.rcv_nxt {
                        let mut seg = Segment::from_bytes(&data.read_bytes(length as usize)
                            .unwrap());
                        seg.conv = conv;
                        seg.cmd = cmd;
                        seg.frg = frg;
                        seg.wnd = wnd;
                        seg.ts = ts;
                        seg.sn = sn;
                        seg.una = una;
                        self.parse_data(seg);
                    }
                }
            } else if cmd == CMD_WASK {
                // ready to send back CMD_WINS in self.flush
                self.probe |= ASK_TELL;
            } else if cmd == CMD_WINS {

            } else {
                return -3;
            }
        }
        if flag != 0 {
            self.parse_fastack(maxack);
        }

        if self.snd_una >= una {
            if self.cwnd < self.rmt_wnd {
                let mss = self.mss;
                if self.cwnd < self.ssthresh {
                    self.cwnd += 1;
                    self.incr += mss;
                } else {
                    if self.incr < mss {
                        self.incr = mss;
                    }
                    self.incr += (mss * mss) / self.incr + (mss / 16);
                    if (self.cwnd + 1) * mss <= self.incr {
                        self.cwnd += 1;
                    }
                }
                if self.cwnd > self.rmt_wnd {
                    self.cwnd = self.rmt_wnd;
                    self.incr = self.rmt_wnd * mss;
                }
            }
        }
        return 0;
    }

    /// update state (call it repeatedly, every 10ms-100ms), or you can ask
    /// self.check when to call it again (without self.input/send calling).
    /// 'current' - current timestamp in millisec.
    pub fn update(&mut self, current: u32) {
        self.current = current;
        if self.updated == 0 {
            self.updated = 1;
            self.ts_flush = self.current;
        }
        let mut slap = sub_u32(self.current, self.ts_flush);
        if slap >= 10000 || slap < -10000 {
            self.ts_flush = self.current;
            slap = 0;
        }
        if slap >= 0 {
            self.ts_flush += self.interval;
            if self.current >= self.ts_flush {
                self.ts_flush = self.current + self.interval;
            }
            self.flush();
        }
    }

    /// determines when should you invoke self.update:
    /// returns when you should invoke ikcp_update in millisec, if there
    /// is no self.input/send calling. you can call self.update in that
    /// time, instead of call update repeatly.
    /// Important to reduce unnacessary self.update invoking. use it to
    /// schedule self.update (eg. implementing an epoll-like mechanism,
    /// or optimize self.update when handling massive kcp connections)
    pub fn check(&mut self, current: u32) -> u32 {
        let mut ts_flush = self.ts_flush;
        let mut tm_flush: i32 = i32::MAX;
        let mut tm_packet: i32 = i32::MAX;
        let mut minimal: u32 = u32::MIN;
        if self.updated == 0 {
            return current;
        }
        let slab = sub_u32(current, ts_flush);
        if slab >= 10000 || slab < -10000 {
            ts_flush = current;
        }
        if current >= ts_flush {
            return current;
        }
        for seg in &self.snd_buf {
            let diff = sub_u32(seg.resendts, current);
            if diff <= 0 {
                return current;
            }
            if diff < tm_packet {
                tm_packet = diff;
            }
        }
        minimal = tm_packet as u32;
        tm_flush = sub_u32(ts_flush, current);
        if tm_packet >= tm_flush {
            minimal = tm_flush as u32;
        }
        if minimal >= self.interval {
            minimal = self.interval;
        }
        return current + minimal;
    }

    /// SetMtu changes MTU size, default is 1400
    pub fn set_mtu(&mut self, mtu: isize) -> isize {
        let mtu_u32 = mtu as u32;
        if mtu < 50 || mtu_u32 < OVERHEAD {
            return -1;
        }
        self.mtu = mtu_u32;
        self.mss = mtu_u32 - OVERHEAD;
        self.buffer = ByteBuffer::with_capacity(((mtu_u32 + OVERHEAD) * 3) as usize);
        return 0;
    }

    /// NoDelay options
    /// fastest: self.no_delay(kcp, 1, 20, 2, 1)
    /// nodelay: 0:disable(default), 1:enable
    /// interval: internal update timer interval in millisec, default is 100ms
    /// resend: 0:disable fast resend(default), 1:enable fast resend
    /// nc: 0:normal congestion control(default), 1:disable congestion control
    pub fn no_delay(&mut self, nodelay: isize, interval: isize, resend: isize, nc: isize) -> isize {
        if nodelay >= 0 {
            self.nodelay = nodelay as u32;
            self.rx_minrto = if nodelay != 0 {
                RTO_NDL
            } else {
                RTO_MIN
            };
        }
        if interval >= 0 {
            self.interval = if interval > 5000 {
                5000
            } else if interval < 10 {
                10
            } else {
                interval as u32
            };
        }
        if resend >= 0 {
            self.fastresend = resend as i32;
        }
        if nc >= 0 {
            self.nocwnd = nc as i32;
        }
        return 0;
    }

    /// set maximum window size: sndwnd=32, rcvwnd=32 by default
    pub fn wnd_size(&mut self, sndwnd: isize, rcvwnd: isize) -> isize {
        if sndwnd > 0 {
            self.snd_wnd = sndwnd as u32;
        }
        if rcvwnd > 0 {
            self.rcv_wnd = rcvwnd as u32;
        }
        return 0;
    }

    /// return the number of packet is waiting to be sent
    pub fn wait_snd(&self) -> isize {
        return (self.snd_buf.len() + self.snd_queue.len()) as isize;
    }

    /// even -> sn odd -> ts
    fn ack_push(&mut self, sn: u32, ts: u32) {
        self.acklist.push(sn);
        self.acklist.push(ts);
    }

    /// get sn and ts from acklist
    fn ack_get(&self, p: usize) -> (u32, u32) {
        (self.acklist[p * 2], self.acklist[p * 2 + 1])
    }

    fn parse_data(&mut self, new_seg: Segment) {
        let sn = new_seg.sn;
        if sn >= (self.rcv_nxt + self.rcv_wnd) || sn < self.rcv_nxt {
            return;
        }
        let length = self.rcv_buf.len();
        for i in (0..length).rev() {
            let tsn = self.rcv_buf[i].sn;
            if sn == tsn {
                // repeat and discard
                break;
            }
            if sn > tsn {
                if i + 1 <= length {
                    self.rcv_buf.insert(i + 1, new_seg);
                } else {
                    self.rcv_buf.push_back(new_seg);
                }
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
            let tsn = self.snd_buf[i].sn;
            if sn == tsn {
                self.snd_buf.remove(i);
                break;
            }
            if sn < tsn {
                break;
            }
        }
    }

    fn parse_fastack(&mut self, sn: u32) {
        if sn < self.snd_una || sn >= self.snd_nxt {
            return;
        }
        for seg in &mut self.snd_buf {
            if sn < seg.sn {
                break;
            } else if sn != seg.sn {
                seg.fastack += 1;
            }
        }
    }

    fn parse_una(&mut self, una: u32) {
        for i in 0..self.snd_buf.len() {
            if una > self.snd_buf[i].sn {
                self.snd_buf.remove(i);
            } else {
                break;
            }
        }
    }

    fn wnd_unused(&mut self) -> i32 {
        if self.rcv_queue.len() < self.rcv_wnd as usize {
            return sub_u32(self.rcv_wnd, self.rcv_queue.len() as u32);
        }
        0
    }

    fn flush(&mut self) {
        if self.updated == 0 {
            return;
        }
        let (current, mut change, mut lost) = (self.current, 0, false);
        let mut seg = Segment::new();
        seg.conv = self.conv;
        seg.cmd = CMD_ACK;
        seg.wnd = self.wnd_unused() as u32;
        seg.una = self.rcv_nxt;

        // flush ack
        for i in 0..self.acklist.len() / 2 {
            let size = self.buffer.get_wpos();
            if size as u32 + OVERHEAD > self.mtu {
                if let Some(output) = self.output {
                    output(&mut self.buffer, size);
                    self.buffer.clear();
                }
            }
            let pair = self.ack_get(i);
            seg.sn = pair.0;
            seg.ts = pair.1;
            seg.encode(&mut self.buffer);
        }
        self.acklist.truncate(0);
        // probe window size (if remote window size equals zero)
        if self.rmt_wnd == 0 {
            if self.probe_wait == 0 {
                self.probe_wait = PROBE_INIT;
                self.ts_probe = self.current + PROBE_INIT;
            } else {
                if self.current >= self.ts_probe {
                    if self.probe_wait < PROBE_INIT {
                        self.probe_wait = PROBE_INIT
                    }
                    self.probe_wait += self.probe_wait / 2;
                    if self.probe_wait > PROBE_LIMIT {
                        self.probe_wait = PROBE_LIMIT;
                    }
                    self.ts_probe = self.current + self.probe_wait;
                    self.probe |= ASK_SEND;
                }
            }
        } else {
            self.ts_probe = 0;
            self.probe_wait = 0;
        }

        // flush window probing commands
        if (self.probe & ASK_SEND) != 0 {
            seg.cmd = CMD_WASK;
            let size = self.buffer.get_wpos();
            if size as u32 + OVERHEAD > self.mtu {
                if let Some(output) = self.output {
                    output(&mut self.buffer, size);
                    self.buffer.clear();
                }
            }
            seg.encode(&mut self.buffer);
        }
        if (self.probe & ASK_TELL) != 0 {
            seg.cmd = CMD_WINS;
            let size = self.buffer.get_wpos();
            if size as u32 + OVERHEAD > self.mtu {
                if let Some(output) = self.output {
                    output(&mut self.buffer, size);
                    self.buffer.clear();
                }
            }
            seg.encode(&mut self.buffer);
        }
        self.probe = 0;

        // calculate window size
        let mut cwnd = min(self.snd_wnd, self.rmt_wnd);
        if self.nocwnd == 0 {
            cwnd = min(self.cwnd, cwnd);
        }
        loop {
            match self.snd_queue.pop_front() {
                Some(mut seg) => {
                    if self.snd_nxt >= self.snd_una + cwnd {
                        break;
                    }
                    seg.conv = self.conv;
                    seg.cmd = CMD_PUSH;
                    seg.ts = current;
                    seg.sn = self.snd_nxt;
                    seg.una = self.rcv_nxt;
                    seg.resendts = current;
                    seg.rto = self.rx_rto;
                    seg.fastack = 0;
                    seg.xmit = 0;
                    self.snd_buf.push_back(seg);
                    self.snd_nxt += 1;
                }
                None => break,
            }
        }

        // calculate resent
        let resent = if self.fastresend <= 0 {
            u32::MAX
        } else {
            self.fastresend as u32
        };
        let rtomin = if self.nodelay != 0 {
            0
        } else {
            self.rx_rto >> 3
        };

        // flush data segments
        for segment in &mut self.snd_buf {
            let mut needsend = false;
            if seg.xmit == 0 {
                needsend = true;
                segment.xmit += 1;
                segment.rto = self.rx_rto;
                segment.resendts = current + self.rx_rto + rtomin;
            } else if current >= segment.resendts {
                needsend = true;
                segment.xmit += 1;
                self.xmit += 1;
                if self.nodelay == 0 {
                    segment.rto += self.rx_rto;
                } else {
                    segment.rto += self.rx_rto / 2;
                }
                segment.rto = min(segment.rto, 8 * self.rx_rto);
                segment.resendts = current + segment.rto;
                lost = true;
            } else if segment.fastack >= resent {
                needsend = true;
                segment.xmit += 1;
                segment.fastack = 0;
                segment.resendts = current + segment.rto;
                change += 1;
            } else if segment.fastack > 0 && self.snd_queue.len() == 0 {
                needsend = true;
                segment.xmit += 1;
                segment.fastack = 0;
                segment.resendts = current + segment.rto;
                change += 1;
            }
            if needsend {
                segment.ts = current;
                segment.wnd = seg.wnd;
                segment.una = self.rcv_nxt;
                let size = self.buffer.get_wpos();
                let need = OVERHEAD + segment.data.len() as u32;

                if size as u32 + need >= self.mtu {
                    if let Some(output) = self.output {
                        output(&mut self.buffer, size);
                        self.buffer.clear();
                    }
                }

                segment.encode(&mut self.buffer);
                self.buffer.write_bytes(&segment.data_bytes());

                if segment.xmit >= self.dead_link {
                    self.state = u32::MAX;
                }
            }
        }
        // flash remain segments
        let size = self.buffer.get_wpos();
        if size > 0 {
            if let Some(output) = self.output {
                output(&mut self.buffer, size);
            }
        }
        // update ssthresh
        // rate halving, https://tools.ietf.org/html/rfc6937
        if change != 0 {
            let inflight = self.snd_nxt - self.snd_una;
            self.ssthresh = inflight / 2;
            if self.ssthresh < THRESH_MIN {
                self.ssthresh = THRESH_MIN;
            }
            self.cwnd = self.ssthresh + resent;
            self.incr = self.cwnd * self.mss;
        }

        // congestion control, https://tools.ietf.org/html/rfc5681
        if lost {
            self.ssthresh = cwnd / 2;
            if self.ssthresh < THRESH_MIN {
                self.ssthresh = THRESH_MIN;
            }
            self.cwnd = 1;
            self.incr = self.mss;
        }

        if self.cwnd < 1 {
            self.cwnd = 1;
            self.incr = self.mss;
        }
    }

    fn shrink_buf(&mut self) {
        if let Some(seg) = self.snd_buf.get(0) {
            self.snd_una = seg.sn;
        } else {
            self.snd_una = self.snd_nxt;
        }
    }

    fn update_ack(&mut self, rtt: u32) {
        if self.rx_srtt == 0 {
            self.rx_srtt = rtt;
            self.rx_rttval = rtt / 2;
        } else {
            self.rx_rttval = (self.rx_rttval * 3 + max(self.rx_srtt, rtt) -
                              min(self.rx_srtt, rtt)) / 4;
            self.rx_srtt = max((self.rx_srtt * 7 + rtt) / 8, 1);
        }
        self.rx_rto = min(max(self.rx_minrto, (self.rx_srtt + max(1, self.rx_rttval * 4))),
                          RTO_MAX);
    }
}

fn sub_u32(a: u32, b: u32) -> i32 {
    a.wrapping_sub(b) as i32
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
