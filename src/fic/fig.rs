use bitvec::{field::BitField, order::Lsb0, view::BitView};
use core::fmt::Debug;

#[derive(Debug)]
pub struct Fig {
    pub header: FigHeader,
    pub kind: FigKind,
}

#[derive(Debug)]
pub struct FigHeader {
    pub kind: u8,
    pub len: usize,
}

#[derive(Debug)]
pub enum FigKind {
    Unknown,
    Type1(Type1)
}

#[derive(Debug)]
pub struct Type1 {
    label: String,
    purpose: LabelPurpose,
}

#[derive(Debug)]
enum LabelPurpose {
    Unknown,
    Ensemble { EId: u16 },
    ProgrammeService { SId: u16 },
    DataService { SId: u32 },
    ServiceComponent { SId: u32, PD: bool, Rfa: u8, SCIdS: u8 },
}

impl Fig {
    pub fn push_data(&mut self, bytes: Vec<u8>) {
        self.kind.push_data(bytes)
    }
}

impl FigKind {
    pub fn push_data(&mut self, bytes: Vec<u8>) {
        match self {
            FigKind::Type1(fig1) => fig1.push_data(bytes),
            _ => return,
        }
    }
}

impl Type1 {
    pub fn push_data(&mut self, bytes: Vec<u8>) {
        let header = bytes[0].view_bits::<Lsb0>();
        let extn: u8 = header[0..3].load_be();
        let oe: u8 = header[3..4].load_be();
        let charset: u8 = header[4..8].load_be();
        self.purpose = match extn {
            0 => Type1::ensemble(&bytes),
            1 => Type1::programme_service(&bytes),
            4 => Type1::service_component(&bytes),
            5 => Type1::data_service(&bytes),
            _ => LabelPurpose::Unknown,
        };
        self.label = match extn {
            0 => String::from_utf8(bytes[3..19].to_vec()).unwrap(),
            1 => String::from_utf8(bytes[3..19].to_vec()).unwrap(),
            4 => {
                let data = bytes[1].view_bits::<Lsb0>();
                let PD: u8 = data[7..8].load_be();
                if PD != 0 {
                    String::from_utf8(bytes[6..22].to_vec()).unwrap()
                }
                else {
                    String::from_utf8(bytes[4..20].to_vec()).unwrap()
                }
            },
            5 => String::from_utf8(bytes[5..21].to_vec()).unwrap(),
            _ => "".to_owned(),
        };
    }

    fn ensemble(bytes: &Vec<u8>) -> LabelPurpose {
        let data = bytes[1..2].view_bits::<Lsb0>();
        let eid = data[0..16].load_be();
        LabelPurpose::Ensemble { EId: eid }
    }

    fn programme_service(bytes: &Vec<u8>) -> LabelPurpose {
        let data = bytes[1..3].view_bits::<Lsb0>();
        let SId = data[0..16].load_be();
        LabelPurpose::ProgrammeService { SId }
    }

    fn service_component(bytes: &Vec<u8>) -> LabelPurpose {
        let data = bytes[1].view_bits::<Lsb0>();
        let SCIdS = data[0..4].load_be();
        let Rfa = data[4..7].load_be();
        let PD: u8 = data[7..8].load_be();
        let SId;
        if PD == 1 {
            let data = bytes[2..6].view_bits::<Lsb0>();
            SId = data[0..32].load_be();
        }
        else {
            let data = bytes[2..4].view_bits::<Lsb0>();
            SId = data[0..16].load_be();
        }
        LabelPurpose::ServiceComponent { SId, Rfa, SCIdS, PD: PD != 0 }
    }

    fn data_service(bytes: &Vec<u8>) -> LabelPurpose {
        let data = bytes[1..5].view_bits::<Lsb0>();
        let SId = data[0..32].load_be();
        LabelPurpose::DataService { SId }        
    }

}

pub fn fig_header(byte: u8) -> Option<Fig> {
    let bits = byte.view_bits::<Lsb0>();
    let kind: u8 = bits[5..8].load_be();
    let len = bits[0..5].load_be();
    if kind > 7 {
        return None;
    }
    if len > 30 {
        return None;
    }
    Some(match kind {
        0 => fig_unknown(kind, len),
        1 => fig_1(len),
        _ => fig_unknown(kind, len),
    })
}

fn fig_unknown(kind: u8, len: usize) -> Fig {
    Fig { header: FigHeader { kind, len }, kind: FigKind::Unknown }
}

fn fig_1(len: usize) -> Fig {
    Fig { header: FigHeader { kind: 1, len }, kind: FigKind::Type1(Type1 { purpose: LabelPurpose::Unknown, label: "".to_owned() }) }
}
