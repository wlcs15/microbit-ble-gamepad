pub mod audio;
pub mod display;

use microbit_bsp::embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Pin, Pull},
    saadc,
    Peri,
};

bind_interrupts!(pub struct Irqs {
    SAADC => saadc::InterruptHandler;
});

pub fn to_button(pin: Peri<'static, impl Pin>) -> Input<'static> {
    Input::new(pin, Pull::Up)
}
