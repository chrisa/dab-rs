use std::thread::JoinHandle;

use crate::fic::ensemble::Service;

pub mod file;
pub mod wavefinder;

pub trait Source {
    fn run(&self) -> JoinHandle<()>;
    fn select_service(&mut self, service: &Service);
}
