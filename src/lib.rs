mod cpu;
mod input;
pub use cpu::Emulator;
pub use cpu::{SCREEN_HEIGHT, SCREEN_WIDTH};
pub use input::{process_key, KeyState};
