pub const LED_COUNT: i32 = 30 as i32;     // Number of LED pixels.
pub const LED_PIN: i32 = 21 as i32;      // GPIO pin connected to the pixels (must support PWM!).
pub const LED_FREQ_HZ: u32 = 800000 as u32;  // LED signal frequency in hertz (usually 800khz)
pub const LED_DMA: i32 = 10 as i32;       // DMA channel to use for generating signal (try 5)
pub const LED_BRIGHTNESS: u8 = 255 as u8;     // Set to 0 for darkest and 255 for brightest
pub const LED_INVERT: i32 = 0 as i32;   // True to invert the signal (when using NPN transistor level shift)
pub const LED_RENDER_WAIT: u64 = 0 as u64;
pub const LED_WSHIFT: u8 = 0 as u8;
pub const LED_RSHIFT: u8 = 0 as u8;
pub const LED_GSHIFT: u8 = 0 as u8;
pub const LED_BSHIFT: u8 = 0 as u8;

pub const CONTROLLER_NAME: &str = "Test Controller";
pub const MAIN_PORT: u16 = 8080 as u16;
pub const SETUP_PORT: u16 = 37322 as u16;
pub const LED_PER_ROW: i64 = 30 as i64;
pub const NUM_ROW: i64 = 1 as i64;
pub const BYTES_PER_LED: i64 = 3 as i64;

pub const UDP_MAX_PACKET_SIZE: u32 = 65507 as u32;