#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m::interrupt::{CriticalSection, Mutex};
use nrf52832_hal::{
    self as hal,
    pac::{Peripherals, UARTE0},
};
use rtt_target::rprintln;

use hal::pac::interrupt;

#[repr(C)]
struct Packet {
    payload: [u8; 5],
}

static mut RXD: [Packet; 10] = [Packet { payload: [0; 5] }; 10];

static P: Mutex<RefCell<Option<Peripherals>>> = Mutex::new(RefCell::new(None));

#[interrupt]
fn UARTE0_UART0() {
    let cs = unsafe { CriticalSection::new() };
    if let Some(peripherals) = P.borrow(&cs).take() {
        if peripherals.UARTE0.events_rxstarted.read().bits() == 1 {
            update_rxd_ptr(&peripherals.UARTE0);
        }
    }
}

#[entry]
fn main() -> ! {
    rtt_target::rtt_init_print!();
    rprintln!("*** Rust-powered GPS tracker ***");

    let peripherals = hal::pac::Peripherals::take().unwrap();

    rprintln!("Starting HF clock...");
    let clock = peripherals.CLOCK;
    clock.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });

    rprintln!("Initialising UART...");
    let uart = peripherals.UARTE0;
    uart.psel.rxd.write(|w| {
        w.pin()
            .variant(3)
            .connect()
            .variant(nrf52832_hal::pac::uarte0::psel::rxd::CONNECT_A::CONNECTED)
    });
    uart.baudrate.write(|w| w.baudrate().baud9600());
    uart.intenset
        .write(|w| w.rxstarted().set_bit().error().set_bit());
    unsafe { hal::pac::NVIC::unmask(hal::pac::Interrupt::UARTE0_UART0) };

    uart.shorts.write(|w| w.endrx_startrx().set_bit());
    update_rxd_ptr(&uart);
    uart.rxd.maxcnt.write(|w| w.maxcnt().variant(5));

    {
        let cs = unsafe { CriticalSection::new() };
        P.borrow(&cs).replace(Some(peripherals));
    }

    loop {}
}

fn update_rxd_ptr(uart: &UARTE0) {
    static mut WRITE: usize = 0;

    uart.rxd.ptr.write(|w| {
        w.ptr()
            .variant(core::ptr::addr_of!(unsafe { RXD[WRITE] }) as u32)
    });
    unsafe { WRITE = (WRITE + 1) % 10 };
}
