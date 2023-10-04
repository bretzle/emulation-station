pub mod coprocessor;
pub mod cpu;
pub mod decoder;
mod interpreter;
pub mod memory;
pub mod state;

pub static mut DEBUG: bool = false;