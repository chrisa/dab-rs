use std::thread::JoinHandle;

pub mod file;
pub mod wavefinder;

pub trait Source {
    fn run(&self) -> JoinHandle<()>;
}
