#![no_std]
#![no_main]

use bleps::{
    ad_structure::{create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE},
    att::Uuid,
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    Addr, Ble, HciConnector,
};
use cortex_m::peripheral::NVIC;
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, pac, peripherals, rng};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::{Error, ErrorKind, ErrorType, Read, Write};
use interrupt::InterruptExt as _;
use nrf_sdc::{
    mpsl::{mpsl_init, mpsl_run, Config as MpslConfig, LfClock},
    raw,
    sdc::{sdc_hci_read, sdc_hci_write, sdc_init, Config as SdcConfig},
    Error as SdcError,
};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => embassy_nrf::rng::InterruptHandler<peripherals::RNG>;
    SWI0_EGU0 => nrf_sdc::mpsl::InterruptHandler;
});

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
    let i = interrupt::POWER_CLOCK;
    info!("POWER_CLOCK IRQ enabled: {}", NVIC::is_enabled(i));
    info!("POWER_CLOCK IRQ active: {}", NVIC::is_active(i));
    info!("POWER_CLOCK IRQ pending: {}", NVIC::is_pending(i));

    let r = unsafe { &*pac::CLOCK::ptr() };
    unsafe { r.intenclr.write(|w| w.bits(0xFFFFFFFF)) };

    print_regs();

    unsafe { MPSL_IRQ_CLOCK_Handler() };
    info!("DONE POWER_CLOCK IRQ");
}

fn current_millis() -> u64 {
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

#[embassy_executor::task]
async fn mpsl_task() {
    loop {
        mpsl_run().await
    }
}

#[embassy_executor::main]
async fn main(_s: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    //config.gpiote_interrupt_priority = interrupt::Priority::P2;
    //config.time_interrupt_priority = interrupt::Priority::P2;
    // config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;

    //print_regs();

    let p = embassy_nrf::init(config);

    interrupt::RTC0.set_priority(interrupt::Priority::P0);
    interrupt::RADIO.set_priority(interrupt::Priority::P0);
    interrupt::TIMER0.set_priority(interrupt::Priority::P0);
    interrupt::POWER_CLOCK.set_priority(interrupt::Priority::P4);
    interrupt::SWI0_EGU0.set_priority(interrupt::Priority::P4);

    info!("Init mpsl");

    //let config = MpslConfig {
    //    source: LfClock::Rc,
    //    rc_ctiv: 16,
    //    rc_temp_ctiv: 2,
    //    accuracy_ppm: 250,
    //};

    let config = MpslConfig {
        source: LfClock::Xtal,
        rc_ctiv: 0,
        rc_temp_ctiv: 0,
        accuracy_ppm: 250,
    };
    mpsl_init(config, Irqs).unwrap();
    //let i = interrupt::POWER_CLOCK;
    //unsafe { NVIC::mask(i) };
    // let res = sys::mpsl_clock_hfclk_request(Some(hfclk_callback));
    // info!("res {}", res);
    Timer::after(Duration::from_millis(10)).await;
    loop {
        info!("Hello");
        Timer::after(Duration::from_millis(300)).await;
    }
    print_regs();
    let mut rng = rng::Rng::new(p.RNG, Irqs);
    rng.set_bias_correction(true);
    let mut seed = [0u8; 32];
    rng.blocking_fill_bytes(&mut seed);
    let config = SdcConfig { seed };
    sdc_init(config).unwrap();

    let connector = SdHci;
    info!("Creating connector!");
    Timer::after(Duration::from_millis(2000)).await;
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
        info!("Writing {} bytes", buf.len());
        sdc_hci_write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
