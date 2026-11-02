pub trait HardwareBackend: Send + Sync {
    fn get_keyboard_brightness(&self) -> Result<u8, String>;

    fn set_keyboard_brightness(&self, value: u8) -> Result<(), String>;

    fn get_keyboard_color(&self) -> Result<(u8, u8, u8), String>;

    fn set_keyboard_color(&self, r: u8, g: u8, b: u8) -> Result<(), String>;
}
