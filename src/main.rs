#![no_std]
#![no_main]

use core::cell::Cell;

use cortex_m_rt::entry;
use critical_section::Mutex;
use nrf52832_hal::{
    self as hal,
    pac::{Peripherals, UARTE0},
};
use rtt_target::rprintln;

use hal::pac::interrupt;

#[repr(C)]
#[derive(Copy, Clone)]
struct Packet {
    payload: [u8; 5],
}

static mut RXD: [Packet; 10] = [Packet { payload: [0; 5] }; 10];
static WRITE: Mutex<Cell<isize>> = Mutex::new(Cell::new(0));
static READ: Mutex<Cell<isize>> = Mutex::new(Cell::new(0));

static P: Mutex<Cell<Option<Peripherals>>> = Mutex::new(Cell::new(None));

#[interrupt]
fn UARTE0_UART0() {
    critical_section::with(|cs| {
        if let Some(peripherals) = P.borrow(cs).take() {
            if peripherals.UARTE0.events_rxstarted.read().bits() == 1 {
                rprintln!("RXSTARTED");
                update_rxd_ptr(&peripherals.UARTE0);
                peripherals.UARTE0.events_rxstarted.reset();
            } else if peripherals.UARTE0.events_error.read().bits() == 1 {
                rprintln!("ERROR");
                peripherals
                    .UARTE0
                    .tasks_flushrx
                    .write(|w| unsafe { w.bits(1) });
            }
        }
    })
}

#[entry]
fn main() -> ! {
    rtt_target::rtt_init_print!();
    rprintln!("*** Rust-powered GPS tracker ***");

    let peripherals = hal::pac::Peripherals::take().unwrap();

    rprintln!("Starting HF clock...");
    let clock = &peripherals.CLOCK;
    clock.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });

    rprintln!("Initialising UART...");
    let uart = &peripherals.UARTE0;
    uart.psel.rxd.write(|w| {
        w.pin()
            .variant(3)
            .connect()
            .variant(nrf52832_hal::pac::uarte0::psel::rxd::CONNECT_A::CONNECTED)
    });
    uart.baudrate.write(|w| w.baudrate().baud9600());
    uart.intenset
        .write(|w| w.rxstarted().set_bit().error().set_bit().endrx().set_bit());
    unsafe { hal::pac::NVIC::unmask(hal::pac::Interrupt::UARTE0_UART0) };

    uart.shorts.write(|w| w.endrx_startrx().set_bit());
    update_rxd_ptr(&uart);
    uart.rxd.maxcnt.write(|w| w.maxcnt().variant(5));

    critical_section::with(|cs| P.borrow(cs).replace(Some(peripherals)));

    loop {}
}

fn update_rxd_ptr(uart: &UARTE0) {
    critical_section::with(|cs| {
        let write_offset = WRITE.borrow(cs).get();
        uart.rxd.ptr.write(|w| {
            w.ptr()
                .variant(unsafe { RXD.as_ptr().offset(write_offset) } as u32)
        });
        WRITE.borrow(cs).set((write_offset + 1) % 10);
    });
}
