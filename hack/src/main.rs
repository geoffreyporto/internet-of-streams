#![no_main]
#![no_std]

#![allow(unused_imports)]

// Built in dependencies
use core::fmt::Write;

// Crates.io dependencies
use dw1000::{DW1000 as DW};
use dwm1001::{
    self,
    nrf52832_hal::{
        delay::Delay,
        prelude::*,
        timer::Timer,
        gpio::{Pin, Output, PushPull, Level, p0::P0_17},
        rng::Rng,
        spim::{Spim},
        nrf52832_pac::{
            TIMER0,
            SPIM2,
            RTC0 as RTC0_PERIPHERAL,
            Interrupt,
        },
    },
    dw1000::{
        mac::Address,
    },
    new_dw1000,
    new_usb_uarte,
    UsbUarteConfig,
    DW_RST,
};
use heapless::{String, consts::*};
use nb::{
    block,
    Error as NbError,
};
use rtfm::app;
use ssmarshal::{serialize, deserialize};

// NOTE: Panic Provider
use panic_ramdump as _;

// NOTE: Must explicitly pull in for RTFM
use nrf52832_pac;

// Workspace dependencies
use protocol::DemoMessage;
use uarte_logger::Logger;
use utils::delay;

mod rtc;
mod clocks;

use crate::clocks::{ClocksExt};
use crate::rtc::{Rtc, RtcExt, Started, RtcInterrupt};


#[app(device = nrf52832_pac)]
const APP: () = {
    static mut LED_RED_1: Pin<Output<PushPull>>     = ();
    static mut TIMER:     Timer<TIMER0>             = ();
    static mut LOGGER:    Logger                    = ();
    static mut DW1000:    DW<
                            Spim<SPIM2>,
                            P0_17<Output<PushPull>>,
                            dw1000::Ready,
                          > = ();
    static mut DW_RST_PIN: DW_RST                   = ();
    static mut RANDOM:     Rng                      = ();
    static mut MY_RTC:        Rtc<RTC0_PERIPHERAL, Started>       = ();

    #[init]
    fn init() {
        let timer = device.TIMER0.constrain();
        let pins = device.P0.split();
        let uarte0 = new_usb_uarte(
            device.UARTE0,
            pins.p0_05,
            pins.p0_11,
            UsbUarteConfig::default(),
        );

        let rng = device.RNG.constrain();

        let dw1000 = new_dw1000(
            device.SPIM2,
            pins.p0_16,
            pins.p0_20,
            pins.p0_18,
            pins.p0_17,
        );

        let mut rst_pin = DW_RST::new(pins.p0_24.into_floating_input());

        // Start the clocks
        let clocks = device.CLOCK.constrain();

        let mut delay = Delay::new(core.SYST, clocks);

        rst_pin.reset_dw1000(&mut delay);

        let dw1000 = dw1000.init().unwrap();

        let mut rtc = RtcExt::constrain(device.RTC0);
        rtc.set_prescaler(0xFFF).unwrap();
        rtc.enable_interrupt(RtcInterrupt::Tick);
        // rtc.enable_event(RtcInterrupt::Tick);

        MY_RTC = rtc.enable_counter();
        RANDOM = rng;
        DW_RST_PIN = rst_pin;
        DW1000 = dw1000;
        LOGGER = Logger::new(uarte0);
        TIMER = timer;
        LED_RED_1 = pins.p0_14.degrade().into_push_pull_output(Level::High);
    }

    #[idle(resources = [TIMER, LOGGER, RANDOM, DW1000])]
    fn idle() -> ! {

        loop {
            let mut out: String<U256> = String::new();
            delay(resources.TIMER, 1_000_000);

            write!(&mut out, "HI").unwrap();
            resources.LOGGER.log(&out).unwrap();

            // write!(
            //     &mut out,
            //     "RTC_CTR: 0x{:08X}\r\n",
            //     resources.RTC.get_counter()
            // ).unwrap();
            // resources.LOGGER.log(&out).expect("hello fail");
            // rtfm::pend(Interrupt::RTC0);
        }
    }

    #[interrupt(resources = [MY_RTC, LED_RED_1])]
    fn RTC0() {
        static mut TOGG: bool = false;
        static mut STEP: u8 = 0;

        (*resources.MY_RTC).get_event_triggered(RtcInterrupt::Tick, true);

        // unsafe {
        //     (*RTC0_PERIPHERAL::ptr()).events_tick.modify(|_r, w| {
        //         w.bits(1)
        //     })
        // };

        *STEP += 1;

        if *STEP < 8 {
            return;
        } else {
            *STEP = 0;
        }

        if *TOGG {
            (*resources.LED_RED_1).set_low();
        } else {
            (*resources.LED_RED_1).set_high();
        }

        *TOGG = !*TOGG;
    }
};
