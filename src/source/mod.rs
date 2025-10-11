use std::thread::JoinHandle;

use crate::msc::cif::MainServiceChannel;

pub mod file;
pub mod wavefinder;

pub trait Source {
    fn run(&self) -> JoinHandle<()>;
    fn select_channel(&mut self, channel: &MainServiceChannel);
}
