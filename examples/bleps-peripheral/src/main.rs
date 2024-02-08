#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, pac::Interrupt::SWI2_EGU2, peripherals, rng};
use embassy_time::Instant;
use {defmt_rtt as _, panic_probe as _};

use bleps::{
    ad_structure::{create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE},
    att::Uuid,
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    Addr, Ble, HciConnector,
};
use embedded_io_async::{Error, ErrorKind, ErrorType, Read, Write};
use nrf_softdevice_controller::{
    mpsl::{mpsl_init, Config as MpslConfig},
    raw,
    sdc::{sdc_hci_read, sdc_hci_write, sdc_init, Config as SdcConfig},
    Error as SdcError,
};

bind_interrupts!(struct Irqs {
    RNG => embassy_nrf::rng::InterruptHandler<peripherals::RNG>;
});

#[interrupt]
fn SWI2_EGU2() {
    defmt::info!("IRQ SWI");
    unsafe {
        raw::mpsl_low_priority_process();
    }
}

fn current_millis() -> u64 {
    Instant::now().as_millis()
}

#[embassy_executor::main]
async fn main(_s: Spawner) {
    let p = embassy_nrf::init(Default::default());

    let mut rng = rng::Rng::new(p.RNG, Irqs);
    rng.set_bias_correction(true);
    let mut seed = [0u8; 32];
    rng.blocking_fill_bytes(&mut seed);

    let config = MpslConfig {};
    mpsl_init(config, SWI2_EGU2).unwrap();

    let config = SdcConfig { seed };
    sdc_init(config).unwrap();

    let connector = SdHci;
    info!("Creating connector!");
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
