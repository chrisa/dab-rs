use std::{sync::mpsc::Receiver, thread::JoinHandle};

use crate::{msc::MainServiceChannel, wavefinder::Buffer};

pub mod file;
pub mod wavefinder;

pub trait Source {
    fn exit(&mut self);
    fn run(&mut self) -> (Receiver<Buffer>, JoinHandle<()>);
    fn ready(&self) -> bool;
    fn select_channel(&mut self, channel: &MainServiceChannel);
}
