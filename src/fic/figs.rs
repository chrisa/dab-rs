use modular_bitfield::prelude::*;
use binrw::BinRead;

#[bitfield]
#[derive(BinRead)]
#[br(map = Self::from_bytes)]
pub struct Header {
    Type: B3,
    Length: B5,
}

#[bitfield]
#[derive(BinRead)]
#[br(map = Self::from_bytes)]
pub struct Type1Header {
    Charset: B4,
    OE: B1,
    Extension: B3,
}

#[derive(BinRead)]
#[repr(C, align(1))]
pub struct Type1_0 {
    EId: u16,
    Label: [u8; 16],
    Flag: u16,
}

#[derive(BinRead)]
#[repr(C, align(1))]
pub struct Type1_1 {
    SId: u16,
    Label: [u8; 16],
    Flag: u16,
}

#[bitfield]
#[derive(BinRead)]
#[br(map = Self::from_bytes)]
pub struct Type1_4Header {
    PD: B1,
    Rfa: B3,
    SCIdS: B4,
}

#[derive(BinRead)]
#[repr(C, align(1))]
pub struct Type1_4ProgrammeSId {
    SId: u16,
}

#[derive(BinRead)]
#[repr(C, align(1))]
pub struct Type1_4DataSId {
    SId: u32,
}

#[derive(BinRead)]
pub enum Type1_4SIds {
    Programme(Type1_4ProgrammeSId),
    Data(Type1_4DataSId),
}

#[derive(BinRead)]
#[repr(C, align(1))]
pub struct Type1_4 {
    Header: Type1_4Header,
    SIds: Type1_4SIds,
    Label: [u8; 16],
    Flag: u16,
}

#[derive(BinRead)]
#[repr(C, align(1))]
pub struct Type1_5 {
    SId: u32,
    Label: [u8; 16],
    Flag: u16,
}

#[derive(BinRead)]
pub enum Type1Fields {
    Ext0(Type1_0),
    Ext1(Type1_1),
}

#[derive(BinRead)]
pub struct Fig_1 {
    FigHeader: Header,
    Type1Header: Type1Header,
    Type1Field: Type1Fields,
}
