use std::{collections::HashMap, fmt};

use super::{WF_REQ_SLMEM, WF_REQ_TIMING, WF_REQ_TUNE};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum MessageKind {
    R1,
    R2,
    Tune,
    Timing,
    SlMem,
}

// #[derive(Debug)]
pub struct Message {
    pub kind: MessageKind,
    pub value: u32,
    pub index: u32,
    pub bytes: Box<[u8]>,
    pub size: usize,
    pub async_: bool,
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for i in self.bytes.iter() {
            let string = format!("0x{:02x} ", i);
            s.push_str(&string);
        }
        write!(
            f,
            "{:?} 0x{:04x} 0x{:04x} ({:?}) {:?}",
            self.kind, self.value, self.index, self.size, s
        )
    }
}

pub fn code_for_kind(kind: &MessageKind) -> u32 {
    let kind_map: HashMap<MessageKind, u32> = HashMap::from([
        (MessageKind::R1, 1),
        (MessageKind::R2, 2),
        (MessageKind::Tune, WF_REQ_TUNE),
        (MessageKind::Timing, WF_REQ_TIMING),
        (MessageKind::SlMem, WF_REQ_SLMEM),
    ]);
    *kind_map.get(kind).unwrap()
}

pub fn tune_msg(reg: u32, bits: u8, pll: u8, lband: bool) -> Message {
    let reg_bytes = reg.to_be_bytes();
    let tbuf: [u8; 12] = [
        reg_bytes[3],
        reg_bytes[2],
        reg_bytes[1],
        reg_bytes[0],
        bits,
        0x00,
        pll,
        0x00,
        lband.into(),
        0x00,
        0x00,
        0x10,
    ];
    Message {
        kind: MessageKind::Tune,
        value: 0,
        index: 0,
        bytes: Box::from(tbuf),
        size: tbuf.len(),
        async_: false,
    }
}

pub fn slmem_msg(value: u32, index: u32, buffer: &Vec<u8>) -> Message {
    Message {
        kind: MessageKind::SlMem,
        value,
        index,
        bytes: Box::from(buffer.as_slice()),
        size: buffer.len(),
        async_: false,
    }
}

pub fn mem_write_msg(addr: u16, val: u16) -> Message {
    let addr_bytes = addr.to_be_bytes();
    let val_bytes = val.to_be_bytes();
    let buffer = vec![addr_bytes[1], addr_bytes[0], val_bytes[1], val_bytes[0]];
    slmem_msg(addr as u32, val as u32, &buffer)
}

pub fn timing_msg(buffer: &[u8; 32]) -> Message {
    Message {
        kind: MessageKind::Timing,
        value: 0,
        index: 0,
        bytes: Box::from(buffer.as_slice()),
        size: 32,
        async_: false,
    }
}

pub fn r2_msg() -> Message {
    Message {
        kind: MessageKind::R2,
        value: 0,
        index: 0x80,
        bytes: Box::from([0; 64]),
        size: 64,
        async_: false,
    }
}

pub fn r1_msg() -> Message {
    Message {
        kind: MessageKind::R1,
        value: 0,
        index: 0x80,
        bytes: Box::from([0; 64]),
        size: 64,
        async_: false,
    }
}
