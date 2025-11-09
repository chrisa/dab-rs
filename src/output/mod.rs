use crate::msc::MainServiceChannelFrame;

pub mod mp2header;
pub mod mpeg;
pub mod aac;
mod firecrc;
mod ka9q_rs;
// mod reedsolomon;

pub trait AudioOutput {
    fn init(&mut self, channels: u32, rate: u32);
    fn deinit(&mut self);
    fn output(&mut self, frame: &MainServiceChannelFrame);
}