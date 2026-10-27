use atsamd_hal::prelude::_embedded_hal_blocking_spi_Write;
use feather_m0::{
    Spi,
    hal::delay::Delay,
    hal::ehal::{delay::DelayNs, digital::OutputPin},
    hal::gpio::{PA02, PA04, PB08, PB09, Pin, PushPullOutput},
};

const SOFTWARE_RESET: u8 = 0x01;
const DISPLAY_OFF: u8 = 0x28;
const POSITIVE_GAMMA: u8 = 0xE0;
const NEGATIVE_GAMMA: u8 = 0xE1;
const POWER_CONTROL_1: u8 = 0xC0;
const POWER_CONTROL_2: u8 = 0xC1;
const VCOM_CONTROL: u8 = 0xC5;
const MEMORY_ACCESS: u8 = 0x36;
const PIXEL_FORMAT: u8 = 0x3A;
const INTERFACE_MODE: u8 = 0xB0;
const FRAME_RATE: u8 = 0xB1;
const INVERSION_CONTROL: u8 = 0xB4;
const DISPLAY_FUNCTION: u8 = 0xB6;
const INTERFACE_CONTROL: u8 = 0xE9;
const ADJUSTMENT_CONTROL: u8 = 0xF7;
const SLEEP_OUT: u8 = 0x11;
const DISPLAY_ON: u8 = 0x29;

const COLUMN_ADDRESS_SET: u8 = 0x2A;
const PAGE_ADDRESS_SET: u8 = 0x2B;
const MEMORY_WRITE: u8 = 0x2C;

#[derive(Debug)]
pub enum DisplayError {
    Spi(&'static str),
}

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

pub enum DrawingTask {
    DrawPixel {
        x: u16,
        y: u16,
        color: Color,
    },
    DrawBox {
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        color: Color,
    },
    FillScreen {
        color: Color,
    },
}

pub struct Ili9488 {
    width: u16,
    height: u16,

    dcrs_pin: Pin<PB08, PushPullOutput>,
    led_pin: Pin<PB09, PushPullOutput>,
    reset_pin: Pin<PA04, PushPullOutput>,
    spi: Spi,
}

impl Ili9488 {
    pub fn new(
        spi: Spi,
        dcrs_pin: Pin<PB08, PushPullOutput>,
        led_pin: Pin<PB09, PushPullOutput>,
        reset_pin: Pin<PA04, PushPullOutput>,
    ) -> Self {
        Self {
            width: 480,
            height: 320,
            spi,
            dcrs_pin,
            led_pin,
            reset_pin,
        }
    }

    pub fn init(&mut self, delay: &mut Delay) -> Result<(), DisplayError> {
        self.width = 480;
        self.height = 320;

        self.reset_pin.set_low().unwrap();
        delay.delay_ms(120u32);
        self.reset_pin.set_high();

        self.write_register(SOFTWARE_RESET, &mut [])?;
        delay.delay_ms(120u32);
        self.write_register(DISPLAY_OFF, &mut [])?;

        self.write_register(
            POSITIVE_GAMMA,
            &mut [
                0x00, 0x03, 0x09, 0x08, 0x16, 0x0A, 0x3F, 0x78, 0x4C, 0x09, 0x0A, 0x08, 0x16, 0x1A,
                0x0F,
            ],
        )?;
        self.write_register(
            NEGATIVE_GAMMA,
            &mut [
                0x00, 0x16, 0x19, 0x03, 0x0F, 0x05, 0x32, 0x45, 0x46, 0x04, 0x0E, 0x0D, 0x35, 0x37,
                0x0F,
            ],
        )?;
        self.write_register(POWER_CONTROL_1, &mut [0x17, 0x15])?;
        self.write_register(POWER_CONTROL_2, &mut [0x41])?;
        self.write_register(VCOM_CONTROL, &mut [0x00, 0x12, 0x80])?;
        self.write_register(MEMORY_ACCESS, &mut [0x28])?;
        self.write_register(PIXEL_FORMAT, &mut [0x66])?;
        self.write_register(INTERFACE_MODE, &mut [0x80])?;
        self.write_register(FRAME_RATE, &mut [0xA0])?;
        self.write_register(INVERSION_CONTROL, &mut [0x02])?;
        self.write_register(DISPLAY_FUNCTION, &mut [0x02, 0x02])?;
        self.write_register(INTERFACE_CONTROL, &mut [0x00])?;
        self.write_register(ADJUSTMENT_CONTROL, &mut [0xA9, 0x51, 0x2C, 0x82])?;

        self.write_register(SLEEP_OUT, &mut [])?;
        delay.delay_ms(120u32);
        self.write_register(DISPLAY_ON, &mut [])?;

        Ok(())
    }

    pub fn write_register(&mut self, command: u8, args: &mut [u8]) -> Result<(), DisplayError> {
        let spi = &mut self.spi;

        self.dcrs_pin.set_low();
        spi.write(&[command])
            .map_err(|_| DisplayError::Spi("write register"))?;

        self.dcrs_pin.set_high();
        for arg in args {
            spi.write(&[*arg])
                .map_err(|_| DisplayError::Spi("write register data"))?;
        }

        Ok(())
    }

    pub fn set_drawing_window(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
    ) -> Result<(), DisplayError> {
        self.write_register(
            COLUMN_ADDRESS_SET,
            &mut [
                (x0 >> 8) as u8,
                (x0 & 0xFF) as u8,
                (x1 >> 8) as u8,
                (x1 & 0xFF) as u8,
            ],
        )?;
        self.write_register(
            PAGE_ADDRESS_SET,
            &mut [
                (y0 >> 8) as u8,
                (y0 & 0xFF) as u8,
                (y1 >> 8) as u8,
                (y1 & 0xFF) as u8,
            ],
        )?;
        self.write_register(MEMORY_WRITE, &mut [])?;

        Ok(())
    }

    pub fn format_color(&mut self, color: Color) -> [u8; 3] {
        let red: u8 = ((color.r as u16 * 63 + 127) / 255) as u8;
        let green: u8 = ((color.g as u16 * 63 + 127) / 255) as u8;
        let blue: u8 = ((color.b as u16 * 63 + 127) / 255) as u8;

        [red << 2, green << 2, blue << 2]
    }

    pub fn draw(&mut self, task: DrawingTask) -> Result<(), DisplayError> {
        match task {
            DrawingTask::DrawPixel { x, y, color } => {
                let x = if x >= self.width { self.width - 1 } else { x };
                let y = if y >= self.width { self.width - 1 } else { y };

                self.set_drawing_window(x, y, x, y)?;
                let mut pixel = self.format_color(color);
                let pixel_data: &mut [u8] = &mut pixel[..];
                self.write_register(MEMORY_WRITE, pixel_data)?;
            }
            DrawingTask::DrawBox {
                x0,
                y0,
                x1,
                y1,
                color,
            } => {
                let x0 = if x0 >= self.width { self.width - 1 } else { x0 };
                let y0 = if y0 >= self.width { self.width - 1 } else { y0 };
                let x1 = if x1 >= self.width { self.width - 1 } else { x1 };
                let y1 = if y1 >= self.width { self.width - 1 } else { y1 };

                self.set_drawing_window(x0, y0, x1, y1)?;
                let mut pixel = self.format_color(color);
                let pixel_data: &mut [u8] = &mut pixel[..];
                self.write_register(MEMORY_WRITE, pixel_data)?;
            }
            DrawingTask::FillScreen { color } => {
                self.set_drawing_window(0, 0, self.width - 1, self.height - 1)?;
                let mut pixel = self.format_color(color);
                let pixel_data: &mut [u8] = &mut pixel[..];
                self.write_register(MEMORY_WRITE, pixel_data)?;
            }
        }

        Ok(())
    }
}
