#![no_std]
#![no_main]

use bleps::{
    ad_structure::{create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE},
    att::Uuid,
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    Addr, Ble, HciConnector,
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, pac, pac::Interrupt::SWI5_EGU5, peripherals, rng};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::{Error, ErrorKind, ErrorType, Read, Write};
use interrupt::InterruptExt as _;
use nrf_softdevice_controller::{
    mpsl::{mpsl_init, Config as MpslConfig},
    raw,
    sdc::{sdc_hci_read, sdc_hci_write, sdc_init, Config as SdcConfig},
    Error as SdcError,
};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => embassy_nrf::rng::InterruptHandler<peripherals::RNG>;
});

#[interrupt]
fn SWI5_EGU5() {
    defmt::info!("IRQ SWI5_EGU5");
    unsafe {
        raw::mpsl_low_priority_process();
    }
}

extern "C" {
    static MPSL_IRQ_RADIO_Handler: extern "C" fn() -> ();
    static MPSL_IRQ_TIMER0_Handler: extern "C" fn() -> ();
    static MPSL_IRQ_RTC0_Handler: extern "C" fn() -> ();
    static MPSL_IRQ_CLOCK_Handler: extern "C" fn() -> ();
}

#[interrupt]
fn RADIO() {
    unsafe { MPSL_IRQ_RADIO_Handler() };
}

#[interrupt]
fn TIMER0() {
    unsafe { MPSL_IRQ_TIMER0_Handler() };
}

#[interrupt]
fn RTC0() {
    unsafe { MPSL_IRQ_RTC0_Handler() };
}

#[interrupt]
fn POWER_CLOCK() {
    info!("POWER_CLOCK IRQ");
    unsafe { MPSL_IRQ_CLOCK_Handler() };
    info!("DONE POWER_CLOCK IRQ");
}

fn current_millis() -> u64 {
    info!("GET MILIS");
    Instant::now().as_millis()
}

fn print_regs() {
    let r = unsafe { &*pac::CLOCK::ptr() };
    let val = r.lfclkstat.read().bits();
    info!("LFCLKSTAT: {:x}", val);
    let val = r.hfclkstat.read().bits();
    info!("HFCLKSTAT: {:x}", val);
    let val = r.lfclkrun.read().bits();
    info!("LFCLKRUN: {:x}", val);
    let val = r.lfclksrc.read().bits();
    info!("LFCLKSRC: {:x}", val);
    let val = r.ctiv.read().bits();
    info!("CTIV: {:x}", val);
}

#[embassy_executor::main]
async fn main(_s: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = interrupt::Priority::P2;
    config.time_interrupt_priority = interrupt::Priority::P2;
    interrupt::RTC0.set_priority(interrupt::Priority::P0);
    interrupt::RADIO.set_priority(interrupt::Priority::P0);
    interrupt::TIMER0.set_priority(interrupt::Priority::P0);
    interrupt::POWER_CLOCK.set_priority(interrupt::Priority::P4);
    interrupt::SWI5_EGU5.set_priority(interrupt::Priority::P4);

    print_regs();

    let p = embassy_nrf::init(config);

    info!("Init mpsl");

    let config = MpslConfig {};
    mpsl_init(config, SWI5_EGU5).unwrap();
    print_regs();
    loop {
        info!("Piung");
        Timer::after(Duration::from_millis(300)).await;
    }
    //
    let mut rng = rng::Rng::new(p.RNG, Irqs);
    rng.set_bias_correction(true);
    let mut seed = [0u8; 32];
    rng.blocking_fill_bytes(&mut seed);
    let config = SdcConfig { seed };
    sdc_init(config).unwrap();

    let connector = SdHci;
    info!("Creating connector!");
    //Timer::after(Duration::from_millis(2000)).await;
    info!("Waited");
    let hci = HciConnector::new(connector, current_millis);
    info!("New Connector");
    let mut ble = Ble::new(&hci);
    info!("New BLE");

    let ret = ble.init();
    info!("Init {:?}", defmt::Debug2Format(&ret));

    let local_addr = Addr::from_le_bytes(false, ble.cmd_read_br_addr().unwrap());

    let ret = ble.cmd_set_le_advertising_parameters();
    info!("ADV PARAMS {:?}", defmt::Debug2Format(&ret));

    let ret = ble.cmd_set_le_advertising_data(
        create_advertising_data(&[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
            AdStructure::CompleteLocalName("BLEPS SD"),
        ])
        .unwrap(),
    );
    info!("CREATE ADV DATA {:?}", defmt::Debug2Format(&ret),);
    let ret = ble.cmd_set_le_advertise_enable(true);
    info!("ENABLE ADV {:?}", defmt::Debug2Format(&ret));

    info!("started advertising");
}

pub struct SdHci;

#[derive(defmt::Format, Debug)]
pub struct HciError {
    error: SdcError,
}

impl From<SdcError> for HciError {
    fn from(e: SdcError) -> Self {
        Self { error: e }
    }
}

impl embedded_io::Error for HciError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl embedded_io::ErrorType for SdHci {
    type Error = HciError;
}

impl embedded_io::Read for SdHci {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let r = sdc_hci_read(buf)?;
        Ok(r)
    }
}

impl embedded_io::Write for SdHci {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        sdc_hci_write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
