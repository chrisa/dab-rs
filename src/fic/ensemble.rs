use itertools::Itertools;
use std::collections::HashMap;

use super::fig::{Fig, FigKind, Information, LabelPurpose, ServiceComponent};

pub struct Ensemble {
    id: u16,
    name: String,
    services: HashMap<u32, Service>,
}

pub struct Service {
    id: u32,
    name: String,
    subchannels: HashMap<u8, SubChannel>,
    data_subchannels: HashMap<u16, DataSubChannel>,
}

pub struct SubChannel {
    id: u8,
    primary: bool,
    start: u16,
    bitrate: u16,
    size: u16,
    protlvl: u8,
    prot: Protection,
}

pub struct DataSubChannel {
    id: u16,
    subchid: u8,
    primary: bool,
    start: u16,
    size: u16,
    protlvl: u8,
    opt: u8,
    scca_flag: u8,
    dg: u8,
    dscty: u8,
    packet_addr: u16,
    scca: u16,
    prot: Protection,
}

#[derive(Debug)]
pub enum Protection {
    Unknown,
    EEP,
    UEP,
}

pub fn new_ensemble() -> Ensemble {
    Ensemble {
        id: 0,
        name: "Unknown".to_owned(),
        services: HashMap::new(),
    }
}

pub fn new_service(id: u32) -> Service {
    Service {
        id,
        name: "Unknown".to_owned(),
        subchannels: HashMap::new(),
        data_subchannels: HashMap::new(),
    }
}

pub fn new_subchannel(id: u8, primary: bool) -> SubChannel {
    SubChannel {
        id,
        primary,
        start: 0,
        bitrate: 0,
        size: 0,
        protlvl: 0,
        prot: Protection::Unknown,
    }
}

pub fn new_data_subchannel(id: u16, primary: bool) -> DataSubChannel {
    DataSubChannel {
        id,
        primary,
        subchid: 0,
        start: 0,
        size: 0,
        protlvl: 0,
        opt: 0,
        scca_flag: 0,
        dg: 0,
        dscty: 0,
        packet_addr: 0,
        scca: 0,
        prot: Protection::Unknown,
    }
}

static uep: [(u16, u16, u8); 64] = [
    (32, 16, 5),
    (32, 21, 4),
    (32, 24, 3),
    (32, 29, 2),
    (32, 35, 1),
    (48, 24, 5),
    (48, 29, 4),
    (48, 35, 3),
    (48, 42, 2),
    (48, 52, 1),
    (56, 29, 5),
    (56, 35, 4),
    (56, 42, 3),
    (56, 52, 2),
    (64, 32, 5),
    (64, 42, 4),
    (64, 48, 3),
    (64, 58, 2),
    (64, 70, 1),
    (80, 40, 5),
    (80, 52, 4),
    (80, 58, 3),
    (80, 70, 2),
    (80, 84, 1),
    (96, 48, 5),
    (96, 58, 4),
    (96, 70, 3),
    (96, 84, 2),
    (96, 104, 1),
    (112, 58, 5),
    (112, 70, 4),
    (112, 84, 3),
    (112, 104, 2),
    (128, 64, 5),
    (128, 84, 4),
    (128, 96, 3),
    (128, 116, 2),
    (128, 140, 1),
    (160, 80, 5),
    (160, 104, 4),
    (160, 116, 3),
    (160, 140, 2),
    (160, 168, 1),
    (192, 96, 5),
    (192, 116, 4),
    (192, 140, 3),
    (192, 168, 2),
    (192, 208, 1),
    (224, 116, 5),
    (224, 140, 4),
    (224, 168, 3),
    (224, 208, 2),
    (224, 232, 1),
    (256, 128, 5),
    (256, 168, 4),
    (256, 192, 3),
    (256, 232, 2),
    (256, 280, 1),
    (320, 160, 5),
    (320, 208, 4),
    (320, 280, 2),
    (384, 192, 5),
    (384, 280, 3),
    (384, 416, 1),
];

impl Ensemble {
    pub fn is_complete(&self) -> bool {
        self.services_labelled() && self.subchannels_contiguous()
    }

    fn services_labelled(&self) -> bool {
        self.services
            .values()
            .map(|s| s.name.as_str())
            .all(|n| n != "Unknown")
    }

    fn subchannels_contiguous(&self) -> bool {
        let subchannels = self
            .services
            .values()
            .flat_map(|s| s.subchannels.values())
            .map(|sc| (sc.start, sc.size));

        let data_subchannels = self
            .services
            .values()
            .flat_map(|s| s.data_subchannels.values())
            .map(|dsc| (dsc.start, dsc.size));

        let all = subchannels.chain(data_subchannels);

        !all.sorted_by(|a, b| Ord::cmp(&a.0, &b.0))
            .scan(0u16, |state, sc| {
                if sc.0 != *state {
                    return Some(u16::MAX);
                }
                *state += if sc.1 == 0 { 1 } else { sc.1 };
                Some(sc.0)
            })
            .any(|start| start == u16::MAX)
    }

    pub fn display(&self) {
        println!("Ensemble:");
        println!("{:16} (0x{:04x})", self.name, self.id);
        for service in self
            .services
            .values()
            .sorted_by(|a, b| Ord::cmp(&a.id, &b.id))
        {
            for subchannel in service.subchannels.values() {
                let PS = if subchannel.primary { "Pri" } else { "Sec " };
                println!(
                    "{:16} (0x{:04x}) {} subch={} start={} size={} bitrate={} {:?}",
                    service.name,
                    service.id,
                    PS,
                    subchannel.id,
                    subchannel.start,
                    subchannel.size,
                    subchannel.bitrate,
                    subchannel.prot
                );
            }
            for data_subchannel in service.data_subchannels.values() {
                let PS = if data_subchannel.primary {
                    "Pri"
                } else {
                    "Sec "
                };
                println!(
                    "{:16} (0x{:04x}) {} subch={} SCId={} start={} size={} addr={} {:?}",
                    service.name,
                    service.id,
                    PS,
                    data_subchannel.subchid,
                    data_subchannel.id,
                    data_subchannel.start,
                    data_subchannel.size,
                    data_subchannel.packet_addr,
                    data_subchannel.prot
                );
            }
        }
        println!();
    }

    pub fn add_fig(&mut self, fig: Fig) {
        match fig.kind {
            FigKind::Type0(fig0) => {
                for info in fig0.info {
                    match info {
                        Information::Ensemble { EId, .. } => self.set_id(EId),
                        Information::Service {
                            SId, components, ..
                        } => {
                            self.add_service(new_service(SId));
                            for component in components {
                                match component {
                                    ServiceComponent::StreamAudio { SubChId, PS, .. } => self
                                        .add_service_subchannel(
                                            SId,
                                            new_subchannel(SubChId, PS != 0),
                                        ),
                                    ServiceComponent::PacketData { SCId, PS, .. } => self
                                        .add_service_data_subchannel(
                                            SId,
                                            new_data_subchannel(SCId, PS != 0),
                                        ),
                                    _ => {}
                                }
                            }
                        }
                        Information::SubChannelShort {
                            SubChId,
                            StartAddr,
                            TabIndx,
                            ..
                        } => {
                            if let Some(SId) = self.find_service_for_subchannel(SubChId) {
                                if TabIndx < 64 {
                                    let (BitRate, SubChSz, ProtLvl) = uep[TabIndx as usize];
                                    self.set_service_subchannel_info(
                                        SId,
                                        SubChId,
                                        StartAddr,
                                        BitRate,
                                        SubChSz,
                                        ProtLvl,
                                        0,
                                        Protection::UEP,
                                    );
                                }
                            }
                        }
                        Information::SubChannelLong {
                            SubChId,
                            StartAddr,
                            Opt,
                            ProtLvl,
                            SubChSz,
                        } => {
                            if let Some(SId) = self.find_service_for_subchannel(SubChId) {
                                self.set_service_subchannel_info(
                                    SId,
                                    SubChId,
                                    StartAddr,
                                    0,
                                    SubChSz,
                                    ProtLvl,
                                    Opt,
                                    Protection::EEP,
                                );
                            }
                        }
                        Information::PacketService {
                            SCId,
                            SCCAFlag,
                            DG,
                            DSCTy,
                            SubChId,
                            PacketAddr,
                            SCCA,
                        } => {
                            if let Some(SId) = self.find_service_for_data_subchannel(SCId) {
                                self.set_service_data_subchannel_info(
                                    SId, SCId, SubChId, SCCAFlag, DG, DSCTy, PacketAddr, SCCA,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            FigKind::Type1(fig1) => {
                match fig1.purpose {
                    LabelPurpose::Ensemble { .. } => self.set_name(fig1.label), // assume one ensemble!
                    LabelPurpose::ProgrammeService { SId } => {
                        self.set_service_name(SId as u32, fig1.label)
                    }
                    LabelPurpose::DataService { SId } => self.set_service_name(SId, fig1.label),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_id(&mut self, id: u16) {
        self.id = id;
    }

    pub fn add_service(&mut self, service: Service) {
        self.services.entry(service.id).or_insert(service);
    }

    pub fn set_service_name(&mut self, service_id: u32, name: String) {
        if let Some(service) = self.services.get_mut(&service_id) {
            service.name = name;
        }
    }

    pub fn add_service_subchannel(&mut self, service_id: u32, subchannel: SubChannel) {
        if let Some(service) = self.services.get_mut(&service_id) {
            service
                .subchannels
                .entry(subchannel.id)
                .or_insert(subchannel);
        }
    }

    pub fn add_service_data_subchannel(&mut self, service_id: u32, subchannel: DataSubChannel) {
        if let Some(service) = self.services.get_mut(&service_id) {
            service
                .data_subchannels
                .entry(subchannel.id)
                .or_insert(subchannel);
        }
    }

    pub fn set_service_subchannel_info(
        &mut self,
        service_id: u32,
        subchannel_id: u8,
        start: u16,
        bitrate: u16,
        size: u16,
        protlvl: u8,
        opt: u8,
        prot: Protection,
    ) {
        if let Some(service) = self.services.get_mut(&service_id) {
            if let Some(subchannel) = service.subchannels.get_mut(&subchannel_id) {
                subchannel.start = start;
                subchannel.bitrate = bitrate;
                subchannel.size = size;
                subchannel.protlvl = protlvl;
                subchannel.prot = prot;
                return;
            }
            for data_subchannel in service.data_subchannels.values_mut() {
                if data_subchannel.subchid == subchannel_id {
                    data_subchannel.start = start;
                    data_subchannel.size = size;
                    data_subchannel.protlvl = protlvl;
                    data_subchannel.opt = opt;
                    data_subchannel.prot = prot;
                    return;
                }
            }
        }
    }

    pub fn set_service_data_subchannel_info(
        &mut self,
        service_id: u32,
        subchannel_id: u16,
        SubChId: u8,
        SCCAFlag: u8,
        DG: u8,
        DSCTy: u8,
        PacketAddr: u16,
        SCCA: u16,
    ) {
        if let Some(service) = self.services.get_mut(&service_id) {
            if let Some(data_subchannel) = service.data_subchannels.get_mut(&subchannel_id) {
                data_subchannel.subchid = SubChId;
                data_subchannel.scca_flag = SCCAFlag;
                data_subchannel.dg = DG;
                data_subchannel.dscty = DSCTy;
                data_subchannel.packet_addr = PacketAddr;
                data_subchannel.scca = SCCA;
            }
        }
    }

    pub fn find_service_for_subchannel(&self, SubChId: u8) -> Option<u32> {
        for service in self.services.values() {
            for subchannel in service.subchannels.values() {
                if subchannel.id == SubChId {
                    return Some(service.id);
                }
            }
            for data_subchannel in service.data_subchannels.values() {
                if data_subchannel.subchid == SubChId {
                    return Some(service.id);
                }
            }
        }
        None
    }

    pub fn find_service_for_data_subchannel(&self, SCId: u16) -> Option<u32> {
        for service in self.services.values() {
            for data_subchannel in service.data_subchannels.values() {
                if data_subchannel.id == SCId {
                    return Some(service.id);
                }
            }
        }
        None
    }
}
