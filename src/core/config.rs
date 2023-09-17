#[derive(Default)]
pub enum BootMode {
    #[default]
    Firmware,
    Direct,
}

#[derive(Default)]
pub struct Config {
    pub game_path: String,
    pub boot_mode: BootMode,
}
