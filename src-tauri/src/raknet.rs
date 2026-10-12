//! RakNet: data datagrams (header 0x80-0x8F) into complete messages, with
//! split messages put back together. ACK/NACK and offline packets are skipped.

use crate::net::Endpoint;
use std::collections::HashMap;

type SplitKey = (Endpoint, Endpoint, u16);

#[derive(Default)]
pub struct Reassembler {
    pending: HashMap<SplitKey, (u32, HashMap<u32, Vec<u8>>, f64)>,
}

impl Reassembler {
    /// The complete messages in one UDP datagram.
    pub fn feed(&mut self, src: Endpoint, dst: Endpoint, time: f64, d: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        if d.is_empty() || !(0x80..=0x8f).contains(&d[0]) {
            return out;
        }
        let mut i = 4; // flags + 24-bit sequence number
        while i + 3 <= d.len() {
            let flags = d[i];
            i += 1;
            let reliability = flags >> 5;
            let split = flags & 0x10 != 0;
            let bits = u16::from_be_bytes([d[i], d[i + 1]]) as usize;
            i += 2;
            let n = bits.div_ceil(8);
            if matches!(reliability, 2 | 3 | 4 | 6 | 7) {
                i += 3; // reliable index
            }
            if matches!(reliability, 1 | 4) {
                i += 3; // sequenced index
            }
            if matches!(reliability, 1 | 3 | 4 | 7) {
                i += 4; // ordered index + channel
            }
            let mut part = None;
            if split {
                let Some(h) = d.get(i..i + 10) else { break };
                let count = u32::from_be_bytes([h[0], h[1], h[2], h[3]]);
                let id = u16::from_be_bytes([h[4], h[5]]);
                let index = u32::from_be_bytes([h[6], h[7], h[8], h[9]]);
                part = Some((count, id, index));
                i += 10;
            }
            let Some(body) = d.get(i..i + n) else { break };
            i += n;
            match part {
                None => out.push(body.to_vec()),
                Some((count, id, index)) => {
                    if count == 0 || count > 4096 {
                        continue;
                    }
                    let entry = self.pending.entry((src, dst, id)).or_insert_with(|| (count, HashMap::new(), time));
                    entry.1.insert(index, body.to_vec());
                    if entry.1.len() as u32 == entry.0 {
                        let (count, parts, _) = self.pending.remove(&(src, dst, id)).expect("entry");
                        let mut whole = Vec::new();
                        for k in 0..count {
                            match parts.get(&k) {
                                Some(p) => whole.extend_from_slice(p),
                                None => {
                                    whole.clear();
                                    break;
                                }
                            }
                        }
                        if !whole.is_empty() {
                            out.push(whole);
                        }
                    }
                }
            }
        }
        // Splits whose last piece never came.
        self.pending.retain(|_, v| time - v.2 < 10.0);
        out
    }
}
