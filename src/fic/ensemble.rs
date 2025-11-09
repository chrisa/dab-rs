#![allow(non_snake_case)]

use itertools::Itertools;
use std::collections::HashMap;

use super::fig::{Fig, FigType, Information, LabelPurpose, ServiceComponent};

use crate::msc::tables::{EEPTABLE, EepProf, UEPTABLE, UepProf};

#[derive(Clone)]
pub struct Ensemble {
    id: u16,
    name: String,
    services: HashMap<u32, Service>,
    label_tries: u16,
}

#[derive(Clone, Debug)]
pub struct Service {
    pub id: u32,
    pub name: String,
    audio_subchannels: HashMap<u8, AudioSubChannel>,
    data_subchannels: HashMap<u16, DataSubChannel>,
}

#[derive(Clone, Debug)]
pub struct AudioSubChannel {
    id: u8,
    primary: bool,
    start: u16,
    bitrate: u16,
    size: u16,
    protlvl: usize,
    opt: usize,
    prot: Protection,
    uep_index: usize,
    audio_type: AudioSubChannelType,
}

#[derive(Clone, Debug)]
pub struct DataSubChannel {
    id: u16,
    subchid: u8,
    primary: bool,
    start: u16,
    size: u16,
    protlvl: usize,
    opt: usize,
    scca_flag: u8,
    dg: u8,
    dscty: u8,
    packet_addr: u16,
    scca: u16,
    prot: Protection,
    uep_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
        label_tries: 0,
    }
}

pub fn new_service(id: u32) -> Service {
    Service {
        id,
        name: "Unknown".to_owned(),
        audio_subchannels: HashMap::new(),
        data_subchannels: HashMap::new(),
    }
}

pub fn new_subchannel(id: u8, primary: bool, ascty: u8) -> AudioSubChannel {
    AudioSubChannel {
        id,
        primary,
        start: 0,
        bitrate: 0,
        size: 0,
        protlvl: 0,
        opt: 0,
        prot: Protection::Unknown,
        uep_index: 0,
        audio_type: match ascty {
            0x00 => AudioSubChannelType::DAB,
            0x3f => AudioSubChannelType::DABPlus,
            _ => AudioSubChannelType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioSubChannelType {
    DAB,
    DABPlus,
    Unknown,
}

#[allow(clippy::too_many_arguments)]
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
        uep_index: 0,
    }
}

impl Ensemble {
    pub fn is_complete(&mut self) -> bool {
        let tries = self.increment_tries();
        let empty = self.services.is_empty();
        let service_labels = self.services_labelled();
        let ensemble_label = self.ensemble_labelled();
        tries > 100 || (!empty && service_labels && ensemble_label)
    }

    pub fn find_service_by_id_str(&self, id_str: &str) -> Option<&Service> {
        if let Ok(id) = u32::from_str_radix(id_str, 16) {
            self.services.values().find(|s| s.id == id)
        } else {
            None
        }
    }

    pub fn find_service_by_id(&self, id: u32) -> Option<&Service> {
        self.services.values().find(|s| s.id == id)
    }

    fn increment_tries(&mut self) -> u16 {
        self.label_tries += 1;
        self.label_tries
    }

    fn services_labelled(&self) -> bool {
        self.services
            .values()
            .map(|s| s.name.as_str())
            .all(|n| n != "Unknown")
    }

    fn ensemble_labelled(&self) -> bool {
        self.name != "Unknown"
    }

    pub fn label(&self) -> &str {
        &self.name
    }

    pub fn services(&self) -> Vec<&Service> {
        self.services
            .values()
            .sorted_by(|a, b| Ord::cmp(&a.id, &b.id))
            .collect_vec()
    }

    pub fn display(&self) {
        eprintln!("Ensemble:");
        eprintln!("{:16} (0x{:04x})", self.name, self.id);
        for service in self.services() {
            for subchannel in service.audio_subchannels.values() {
                let PS = if subchannel.primary { "Pri" } else { "Sec " };
                eprintln!(
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
                eprintln!(
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
        eprintln!();
    }

    pub fn add_fig(&mut self, fig: Fig) {
        match fig.figtype {
            FigType::Type0(fig0) => {
                for info in fig0.info {
                    match info {
                        Information::Ensemble { EId, .. } => self.set_id(EId),
                        Information::Service {
                            SId, components, ..
                        } => {
                            self.add_service(new_service(SId));
                            for component in components {
                                match component {
                                    ServiceComponent::StreamAudio { SubChId, PS, ASCTy, .. } => self
                                        .add_service_subchannel(
                                            SId,
                                            new_subchannel(SubChId, PS != 0, ASCTy),
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
                            if let Some(SId) = self.find_service_for_subchannel(SubChId)
                                && TabIndx < 64
                            {
                                let uep = UEPTABLE[TabIndx as usize];
                                self.set_service_subchannel_info(
                                    SId,
                                    SubChId,
                                    StartAddr,
                                    uep.BitRate,
                                    uep.SubChSz,
                                    uep.ProtLvl,
                                    0,
                                    Protection::UEP,
                                    TabIndx as usize,
                                );
                            }
                        }
                        Information::SubChannelLong {
                            SubChId,
                            StartAddr,
                            Opt,
                            ProtLvl,
                            SubChSz,
                        } => {
                            let profile = EEPTABLE[Opt as usize][ProtLvl as usize];
                            let bitrate = match Opt {
                                0 => profile.sizemul * 8,
                                1 => profile.sizemul * 32,
                                u => panic!("unexpected EEP Opt value: {}", u),
                            };

                            if let Some(SId) = self.find_service_for_subchannel(SubChId) {
                                self.set_service_subchannel_info(
                                    SId,
                                    SubChId,
                                    StartAddr,
                                    bitrate,
                                    SubChSz,
                                    ProtLvl,
                                    Opt,
                                    Protection::EEP,
                                    0,
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
            FigType::Type1(fig1) => {
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

    pub fn add_service_subchannel(&mut self, service_id: u32, subchannel: AudioSubChannel) {
        if let Some(service) = self.services.get_mut(&service_id) {
            service
                .audio_subchannels
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

    #[allow(clippy::too_many_arguments)]
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
        uep_index: usize,
    ) {
        if let Some(service) = self.services.get_mut(&service_id) {
            if let Some(subchannel) = service.audio_subchannels.get_mut(&subchannel_id) {
                subchannel.start = start;
                subchannel.bitrate = bitrate;
                subchannel.size = size;
                subchannel.protlvl = protlvl as usize;
                subchannel.opt = opt as usize;
                subchannel.prot = prot;
                subchannel.uep_index = uep_index;
                return;
            }
            for data_subchannel in service.data_subchannels.values_mut() {
                if data_subchannel.subchid == subchannel_id {
                    data_subchannel.start = start;
                    data_subchannel.size = size;
                    data_subchannel.protlvl = protlvl as usize;
                    data_subchannel.opt = opt as usize;
                    data_subchannel.prot = prot;
                    data_subchannel.uep_index = uep_index;
                    return;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        if let Some(service) = self.services.get_mut(&service_id)
            && let Some(data_subchannel) = service.data_subchannels.get_mut(&subchannel_id)
        {
            data_subchannel.subchid = SubChId;
            data_subchannel.scca_flag = SCCAFlag;
            data_subchannel.dg = DG;
            data_subchannel.dscty = DSCTy;
            data_subchannel.packet_addr = PacketAddr;
            data_subchannel.scca = SCCA;
        }
    }

    pub fn find_service_for_subchannel(&self, SubChId: u8) -> Option<u32> {
        for service in self.services.values() {
            for subchannel in service.audio_subchannels.values() {
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

impl Service {
    pub fn label(&self) -> &str {
        &self.name
    }

    // TODO; deal with more than one subchannel
    pub fn subchannel(&self) -> &dyn SubChannel {
        if let Some(subchannel) = self.audio_subchannels.values().next() {
            return subchannel;
        }
        if let Some(subchannel) = self.data_subchannels.values().next() {
            return subchannel;
        }
        panic!("no subchannels?");
    }
}

#[derive(Debug)]
pub enum SubChannelType {
    Audio,
    Data,
}

pub trait SubChannel {
    fn startaddr(&self) -> u16;
    fn size(&self) -> u16;
    fn protection(&self) -> Protection;
    fn subchannel_type(&self) -> SubChannelType;
    fn uep_profile(&self) -> Option<UepProf>;
    fn eep_profile(&self) -> Option<EepProf>;
    fn bitrate(&self) -> u16;
    fn audio_type(&self) -> AudioSubChannelType;
    fn protlvl(&self) -> usize;
    fn opt(&self) -> usize;
    // fn as_any(&self) -> &dyn Any;
}

impl SubChannel for AudioSubChannel {
    fn startaddr(&self) -> u16 {
        self.start
    }
    fn size(&self) -> u16 {
        self.size
    }
    fn protection(&self) -> Protection {
        self.prot
    }
    fn subchannel_type(&self) -> SubChannelType {
        SubChannelType::Audio
    }
    fn uep_profile(&self) -> Option<UepProf> {
        if self.prot == Protection::UEP {
            return Some(UEPTABLE[self.uep_index]);
        }
        None
    }
    fn eep_profile(&self) -> Option<EepProf> {
        if self.prot == Protection::EEP {
            return Some(EEPTABLE[self.opt][self.protlvl]);
        }
        None
    }
    fn bitrate(&self) -> u16 {
        self.bitrate
    }
    fn audio_type(&self) -> AudioSubChannelType {
        self.audio_type
    }
    fn protlvl(&self) -> usize {
        self.protlvl
    }
    fn opt(&self) -> usize {
        self.opt
    }
    // fn as_any(&self) -> &dyn Any {
    //     self
    // }
}

impl SubChannel for DataSubChannel {
    fn startaddr(&self) -> u16 {
        self.start
    }
    fn size(&self) -> u16 {
        self.size
    }
    fn protection(&self) -> Protection {
        self.prot
    }
    fn subchannel_type(&self) -> SubChannelType {
        SubChannelType::Data
    }
    fn uep_profile(&self) -> Option<UepProf> {
        None
    }
    fn eep_profile(&self) -> Option<EepProf> {
        None
    }
    fn bitrate(&self) -> u16 {
        0
    }
    fn audio_type(&self) -> AudioSubChannelType {
        AudioSubChannelType::Unknown
    }
    fn protlvl(&self) -> usize {
        self.protlvl
    }
    fn opt(&self) -> usize {
        self.opt
    }
    // fn as_any(&self) -> &dyn Any {
    //     self
    // }
}
