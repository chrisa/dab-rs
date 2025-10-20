use std::thread::JoinHandle;

use crate::msc::MainServiceChannel;

pub mod file;
pub mod wavefinder;

pub trait Source {
    fn exit(&mut self);
    fn run(&mut self) -> JoinHandle<()>;
    fn ready(&self) -> bool;
    fn select_channel(&mut self, channel: &MainServiceChannel);
}
