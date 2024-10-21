use bitvec::{field::BitField, order::{Lsb0, Msb0}, view::BitView};
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
    Type0(Type0),
    Type1(Type1),
}

#[derive(Debug)]
pub struct Type0 {
    pub info: Vec<Information>,
}

#[derive(Debug)]
pub enum Information {
    Unknown,
    Ensemble { OccChg: u8, CIFCntL: u8, CIFCntH: u8, AlrmFlg: u8, ChgFlg: u8, EId: u16 },
    SubChannelShort { SubChId: u8, StartAddr: u16, TableSw: u8, TabIndx: u8 },
    SubChannelLong { SubChId: u8, StartAddr: u16, Opt: u8, ProtLvl: u8, SubChSz: u16 },
    Service { SId: u32, PD: bool, components: Vec<ServiceComponent> },
    PacketService { SCId: u16, SCCAFlag: u8, DG: u8, DSCTy: u8, SubChId: u8, PacketAddr: u16, SCCA: u16 },
}

#[derive(Debug)]
pub enum ServiceComponent {
    Unknown,
    StreamAudio { ASCTy: u8, SubChId: u8, PS: u8, CAFlg: u8 },
    StreamData { DSCTy: u8, SubChId: u8, PS: u8, CAFlg: u8 },
    FIDC { DSCTy: u8, FIDCId: u8, PS: u8, CAFlg: u8 },
    PacketData { SCId: u16, PS: u8, CAFlg: u8 },
}

#[derive(Debug)]
pub struct Type1 {
    pub label: String,
    pub purpose: LabelPurpose,
}

#[derive(Debug)]
pub enum LabelPurpose {
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
            FigKind::Type0(fig0) => fig0.push_data(bytes),
            FigKind::Type1(fig1) => fig1.push_data(bytes),
            _ => return,
        }
    }
}

pub fn fig_header(byte: u8) -> Option<Fig> {
    let bits = byte.view_bits::<Lsb0>();
    let len = bits[0..5].load_be();
    let kind: u8 = bits[5..8].load_be();
    if kind > 7 {
        return None;
    }
    if len > 30 {
        return None;
    }
    Some(match kind {
        0 => fig_0(len),
        1 => fig_1(len),
        _ => fig_unknown(kind, len),
    })
}

fn fig_unknown(kind: u8, len: usize) -> Fig {
    Fig { header: FigHeader { kind, len }, kind: FigKind::Unknown }
}

fn fig_0(len: usize) -> Fig {
    Fig { header: FigHeader { kind: 0, len }, kind: FigKind::Type0(Type0 { info: vec!(Information::Unknown) }) }
}

fn fig_1(len: usize) -> Fig {
    Fig { header: FigHeader { kind: 1, len }, kind: FigKind::Type1(Type1 { purpose: LabelPurpose::Unknown, label: "".to_owned() }) }
}

impl Type0 {
    pub fn push_data(&mut self, bytes: Vec<u8>) {
        let header = bytes[0].view_bits::<Lsb0>();
        let extn: u8 = header[0..5].load_be();
        let pd: u8 = header[5..6].load_be();
        let oe: u8 = header[6..7].load_be();
        let cn: u8 = header[7..8].load_be();
        self.info = match extn {
            0 => Type0::ensemble(pd, &bytes[1..]),
            1 => Type0::subchannel(pd, &bytes[1..]),
            2 => Type0::service(pd, &bytes[1..]),
            3 => Type0::packet_service_component(pd, &bytes[1..]),
            _ => vec!(Information::Unknown),
        };
        dbg!(&self);
    }

    fn ensemble(pd: u8, bytes: &[u8]) -> Vec<Information> {
        // assert!(bytes.len() == 5);
        let data = bytes.view_bits::<Lsb0>();
        let EId: u16 = data[0..16].load_be();
        let ChgFlg: u8 = data[16..18].load_be();
        let AlrmFlg: u8 = data[18..19].load_be();
        let CIFCntH: u8 = data[19..24].load_be();
        let CIFCntL: u8 = data[24..32].load_be();
        // let OccChg: u8 = data[32..40].load_be();
        vec!(Information::Ensemble { OccChg: 0, CIFCntL, CIFCntH, AlrmFlg, ChgFlg, EId })
    }

    fn subchannel(pd: u8, bytes: &[u8]) -> Vec<Information> {
        let mut offset = 0;
        let mut subchannels = Vec::new();
        while offset < bytes.len() {            
            let data = bytes[offset..].view_bits::<Msb0>();
            let SubChId: u8 = data[0..6].load_be();
            let StartAddr: u16 = data[6..16].load_be();
            let LongForm: u8 = data[16..17].load_be();
            if LongForm != 0 {
                assert!(bytes[offset..].len() >= 4);
                offset += 4;
                let Opt: u8 = data[17..20].load_be();
                let ProtLvl: u8 = data[20..22].load_be();
                let SubChSz: u16 = data[22..32].load_be();
                subchannels.push(Information::SubChannelLong { SubChId, StartAddr, Opt, ProtLvl, SubChSz });
            }
            else {
                assert!(bytes[offset..].len() >= 3);
                offset += 3;
                let TableSw: u8 = data[17..18].load_be();
                let TabIndx: u8 = data[18..24].load_be();
                subchannels.push(Information::SubChannelShort { SubChId, StartAddr, TableSw, TabIndx });
            }
        }
        subchannels
    }

    fn service(pd: u8, bytes: &[u8]) -> Vec<Information> {
        let mut offset = 0;
        let mut services = Vec::new();
        while offset < bytes.len() {            
            let mut data = bytes[offset..].view_bits::<Msb0>();
            let SId: u32;
            if pd != 0 {
                SId = data[0..32].load_be();
                offset += 4;
            }
            else {
                SId = data[0..16].load_be();
                offset += 2;
            }
            data = bytes[offset..].view_bits::<Msb0>();
            let Local: u8  = data[0..1].load_be();
            let CAId: u8 = data[1..4].load_be();
            let NumSCmp: u8 = data[4..8].load_be();
            offset += 1;
            let mut components = vec!();
            for i in 0..NumSCmp { 
                data = bytes[offset..].view_bits::<Msb0>();
                let TMId: u8 = data[0..2].load_be();
                components.push(match TMId {
                    0 => {
                        let ASCTy: u8 = data[2..8].load_be();
                        let SubChId: u8 = data[8..14].load_be();
                        let PS: u8 = data[14..15].load_be();
                        let CAFlg: u8 = data[15..16].load_be();
                        ServiceComponent::StreamAudio { ASCTy, SubChId, PS, CAFlg }
                    },
                    1 => {
                        let DSCTy: u8 = data[2..8].load_be();
                        let SubChId: u8 = data[8..14].load_be();
                        let PS: u8 = data[14..15].load_be();
                        let CAFlg: u8 = data[15..16].load_be();
                        ServiceComponent::StreamData { DSCTy, SubChId, PS, CAFlg }
                    },
                    2 => {
                        let DSCTy: u8 = data[2..8].load_be();
                        let FIDCId: u8 = data[8..14].load_be();
                        let PS: u8 = data[14..15].load_be();
                        let CAFlg: u8 = data[15..16].load_be();
                        ServiceComponent::FIDC { DSCTy, FIDCId, PS, CAFlg }
                    },
                    3 => {
                        let SCId: u16 = data[2..14].load_be();
                        let PS: u8 = data[14..15].load_be();
                        let CAFlg: u8 = data[15..16].load_be();
                        ServiceComponent::PacketData { SCId, PS, CAFlg }
                    },
                    _ => ServiceComponent::Unknown,
                });
                offset += 2;
            }
            services.push(Information::Service { SId, PD: pd != 0, components })
        }
        services
    }

    fn packet_service_component(pd: u8, bytes: &[u8]) -> Vec<Information> {
        let mut offset = 0;
        let mut service_components = Vec::new();
        while offset < bytes.len() {            
            let mut data = bytes[offset..].view_bits::<Msb0>();
            let SCId: u16 = data[0..12].load_be();
            let Rfa: u8 = data[12..15].load_be();
            let SCCAFlag: u8 = data[15..16].load_be();
            let DG: u8 = data[16..17].load_be();
            let Rfu: u8 = data[17..18].load_be();
            let DSCTy: u8 = data[18..24].load_be();
            let SubChId: u8 = data[24..30].load_be();
            let PacketAddr: u16 = data[30..40].load_be();
            // let SCCA: u16 = data[40..56].load_be();
            service_components.push(Information::PacketService { SCId, SCCAFlag, DG, DSCTy, SubChId, PacketAddr, SCCA: 0 });
            offset += 5;
        }
        service_components
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
        let data = bytes[1..3].view_bits::<Lsb0>();
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

