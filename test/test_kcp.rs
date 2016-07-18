use std::time::{Duration, SystemTime};
use std::thread;
use fixbuf::ByteBuffer;
use time;
use rand;
use rand::Rng;
use std::vec::Vec;
use kcp::KCP;
use std::rc::Rc;
use std::cell::RefCell;

fn iclock() -> i32 {
    let (s, u): (i64, i64);
    let value: i32;
    s = time::now().to_timespec().sec;
    u = time::now().to_timespec().nsec as i64;
    value = (s * 1000 + (u / 1000)) as i32;
    return value & 0xffffffff;
}

#[derive(Default)]
struct depay_packet {
    pub _prt: ByteBuffer,
    pub _size: isize,
    pub _ts: isize,
}

impl depay_packet {
    fn init(size: isize, src: &ByteBuffer) -> depay_packet {
        let mut depay = depay_packet { ..Default::default() };
        depay._size = size;
        depay._prt.write_bytes(src.to_bytes().as_mut());
        return depay;
    }
}
// type rand = rand::thread_rng();

#[derive(Default)]
struct Latency_simulator {
    current: i32,
    lostrate: isize,
    rttmin: isize,
    rttmax: isize,
    nmax: isize,
    p12: Vec<depay_packet>,
    p21: Vec<depay_packet>,
}

impl Latency_simulator {
    fn new(lostrate: isize, rttmin: isize, rttmax: isize, nmax: isize) -> Latency_simulator {
        let mut latency_simulator = Latency_simulator { ..Default::default() };
        latency_simulator.current = iclock();
        latency_simulator.lostrate = lostrate / 2;
        latency_simulator.rttmin = rttmin / 2;
        latency_simulator.rttmax = rttmax / 2;
        latency_simulator.nmax = nmax;
        latency_simulator
    }

    fn send(&mut self, peer: isize, data: &ByteBuffer, size: isize) -> isize {
        let mut rng = rand::thread_rng();
        let rnd = rng.gen::<isize>();
        if rnd < self.lostrate {
            return 0;
        }
        let mut pkt = depay_packet::init(size, &data);
        self.current = iclock();
        let mut delay = self.rttmin;
        if self.rttmax > self.rttmin {
            delay += rng.gen::<isize>() % (self.rttmax - self.rttmin);
        }
        pkt._ts = self.current as isize + delay as isize;
        if peer == 0 {
            self.p12.push(pkt);
        } else {
            self.p21.push(pkt);
        }
        return 1;
    }

    fn recv(&mut self, peer: isize, data: &mut ByteBuffer, maxsize: isize) -> isize {
        let mut pkt;
        if peer == 0 {
            match self.p21.pop() {
                Some(data) => pkt = data,
                None => return -1,
            }

        } else {
            match self.p12.pop() {
                Some(data) => pkt = data,
                None => return -1,
            }
        };
        self.current = iclock();
        if self.current < pkt._ts as i32 {
            return -2;
        }
        if maxsize < pkt._size {
            return -3;
        }
        data.write_bytes(pkt._prt.to_bytes().as_mut());
        return pkt._size;
    }
}

fn test(mode: isize) {
    let vnet = Rc::new(RefCell::new(Latency_simulator::new(10, 60, 125, 1000)));
	let vnet1 = vnet.clone();
    let mut kcp1 = KCP::new(0x11223344, move |buf, size| {
        vnet1.borrow_mut().send(0, buf, size as isize);
    });
	let vnet2 = vnet.clone();
    let mut kcp2 = KCP::new(0x11223344, move |buf, size| {
		vnet2.borrow_mut().send(0, buf, size as isize);
	});
    let mut current = iclock() as u32;
    let mut slap = current + 20;
    let mut index = 0;
    let mut next = 0;
    let mut sumrtt: u32 = 0;
    let mut count = 0;
    let mut maxrtt = 0;
    // 配置窗口大小：平均延迟200ms，每20ms发送一个包，
    // 而考虑到丢包重发，设置最大收发窗口为128
    kcp1.wnd_size(128, 128);
    kcp2.wnd_size(128, 128);
    if mode == 0 {
        // 默认模式
        kcp1.no_delay(0, 10, 0, 0);
        kcp2.no_delay(0, 10, 0, 0);
    } else if mode == 1 {
        // 普通模式，关闭流控等
        kcp1.no_delay(0, 10, 0, 1);
        kcp2.no_delay(0, 10, 0, 1);
    } else {
        // 启动快速模式
        // 第二个参数 nodelay-启用以后若干常规加速将启动
        // 第三个参数 interval为内部处理时钟，默认设置为 10ms
        // 第四个参数 resend为快速重传指标，设置为2
        // 第五个参数 为是否禁用常规流控，这里禁止
        kcp1.no_delay(1, 10, 2, 1);
        kcp2.no_delay(1, 10, 2, 1);
    }
    let mut buffer: ByteBuffer = ByteBuffer::with_capacity(2000);
    let mut hr: i32;
    let mut ts1 = iclock();
    loop {
        thread::sleep(Duration::from_millis(100));
        current = iclock() as u32;
        kcp1.update(iclock() as u32);
        kcp2.update(iclock() as u32);
        // 每隔 20ms，kcp1发送数据
        while current >= slap {
            let mut buf: ByteBuffer = ByteBuffer::with_capacity(2000);
            buf.write_u32(index);
            index += 1;
            buf.write_u32(current);
            kcp1.send(&mut buf);
            slap += 20;
        }
        // 处理虚拟网络：检测是否有udp包从p1->p2
        loop {
            hr = vnet.borrow_mut().recv(1, &mut buffer, 2000) as i32;
            if hr < 0 {
                break;
            }
            kcp2.input(&mut buffer);
        }
        // 处理虚拟网络：检测是否有udp包从p2->p1
        loop {
            hr = vnet.borrow_mut().recv(0, &mut buffer, 2000) as i32;
            if hr < 0 {
                break;
            }
            kcp1.input(&mut buffer);
        }
        // kcp2接收到任何包都返回回去
        loop {
            hr = kcp2.recv(&mut buffer) as i32;
            if hr < 0 {
                break;
            }
            kcp2.send(&mut buffer);
        }
        // kcp1收到kcp2的回射数据
        loop {
            hr = kcp1.recv(&mut buffer) as i32;
            if hr < 0 {
                break;
            }
            let sn = buffer.read_u32().unwrap();
            let ts = buffer.read_u32().unwrap();
            let rtt = current - ts;

            if sn != next {
                println!("ERROR sn {} <-> {}, {}", count, next, sn);
                return;
            }
            next += 1;
            sumrtt += rtt;
            count += 1;
            if rtt > maxrtt {
                maxrtt = rtt;
            }
            println!("[RECV] mode = {} sn = {} rtt = {}", mode, sn, rtt);
        }
        if next > 100 {
            break;
        }
    }
    ts1 = iclock() - ts1;
    let names = &["default", "normal", "fast"];
    println!("{} mode result {}", names[mode as usize], ts1);
    println!("avgrtt = {} max rtt = {}", (sumrtt / count), maxrtt);
}

#[test]
fn test_network() {
    test(0);
    test(1);
    test(2);
}
