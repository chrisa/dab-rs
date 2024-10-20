pub mod file;
pub mod wavefinder;

pub trait Source {
    fn run(&self);
}