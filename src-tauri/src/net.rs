//! Packet capture: UDP datagrams from a live device or a pcap file.

use std::net::IpAddr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Endpoint {
    pub ip: IpAddr,
    pub port: u16,
}

pub struct Datagram {
    pub time: f64,
    pub src: Endpoint,
    pub dst: Endpoint,
    pub payload: Vec<u8>,
}

/// Where the link layer ends and IP starts, for the link types we can meet.
fn ip_offset(linktype: i32, frame: &[u8]) -> Option<usize> {
    match linktype {
        1 => {
            // Ethernet, maybe with one VLAN tag.
            let et = u16::from_be_bytes([*frame.get(12)?, *frame.get(13)?]);
            if et == 0x8100 { Some(18) } else { Some(14) }
        }
        113 => Some(16), // Linux cooked (SLL), the "any" device
        276 => Some(20), // Linux cooked v2
        0 | 108 => Some(4), // BSD loopback / Npcap loopback
        12 | 101 => Some(0), // raw IP
        _ => None,
    }
}

pub fn parse(linktype: i32, time: f64, frame: &[u8]) -> Option<Datagram> {
    let ip = &frame[ip_offset(linktype, frame)?..];
    let version = ip.first()? >> 4;
    let (src_ip, dst_ip, udp) = match version {
        4 => {
            if *ip.get(9)? != 17 {
                return None;
            }
            let ihl = ((ip[0] & 0x0F) as usize) * 4;
            let s: [u8; 4] = ip.get(12..16)?.try_into().ok()?;
            let d: [u8; 4] = ip.get(16..20)?.try_into().ok()?;
            (IpAddr::from(s), IpAddr::from(d), ip.get(ihl..)?)
        }
        6 => {
            if *ip.get(6)? != 17 {
                return None; // no extension headers expected for game traffic
            }
            let s: [u8; 16] = ip.get(8..24)?.try_into().ok()?;
            let d: [u8; 16] = ip.get(24..40)?.try_into().ok()?;
            (IpAddr::from(s), IpAddr::from(d), ip.get(40..)?)
        }
        _ => return None,
    };
    let sport = u16::from_be_bytes([*udp.first()?, *udp.get(1)?]);
    let dport = u16::from_be_bytes([*udp.get(2)?, *udp.get(3)?]);
    Some(Datagram {
        time,
        src: Endpoint { ip: src_ip, port: sport },
        dst: Endpoint { ip: dst_ip, port: dport },
        payload: udp.get(8..)?.to_vec(),
    })
}

/// Live capture on every interface (Linux "any") or the default device,
/// or a pcap file replayed at its own pace. Calls `sink` for each UDP datagram.
pub fn run(source: Option<String>, realtime: bool, mut sink: impl FnMut(Datagram)) -> Result<(), String> {
    if let Some(path) = source {
        let mut cap = pcap::Capture::from_file(&path).map_err(|e| e.to_string())?;
        let lt = cap.get_datalink().0;
        let mut first: Option<(f64, std::time::Instant)> = None;
        while let Ok(p) = cap.next_packet() {
            let t = p.header.ts.tv_sec as f64 + p.header.ts.tv_usec as f64 / 1e6;
            // Keep the capture's timing so the overlay replays like the real thing.
            let (t0, start) = *first.get_or_insert((t, std::time::Instant::now()));
            let due = std::time::Duration::from_secs_f64((t - t0).max(0.0));
            if let Some(wait) = due.checked_sub(start.elapsed()).filter(|_| realtime) {
                std::thread::sleep(wait);
            }
            if let Some(d) = parse(lt, t, p.data) {
                sink(d);
            }
        }
        return Ok(());
    }
    let device = if cfg!(target_os = "linux") {
        pcap::Device::list()
            .ok()
            .and_then(|l| l.into_iter().find(|d| d.name == "any"))
            .ok_or("no \"any\" capture device")?
    } else {
        pcap::Device::lookup().map_err(|e| e.to_string())?.ok_or("no capture device")?
    };
    let mut cap = pcap::Capture::from_device(device)
        .map_err(|e| e.to_string())?
        .immediate_mode(true)
        .snaplen(65535)
        .open()
        .map_err(|e| e.to_string())?;
    cap.filter("udp", true).map_err(|e| e.to_string())?;
    let lt = cap.get_datalink().0;
    loop {
        match cap.next_packet() {
            Ok(p) => {
                let t = p.header.ts.tv_sec as f64 + p.header.ts.tv_usec as f64 / 1e6;
                if let Some(d) = parse(lt, t, p.data) {
                    sink(d);
                }
            }
            Err(pcap::Error::TimeoutExpired) => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
}
