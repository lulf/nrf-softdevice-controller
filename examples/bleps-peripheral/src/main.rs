#![no_std]
#![no_main]

use bleps::{
    ad_structure::{create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE},
    att::Uuid,
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    Addr, Ble, HciConnector,
};
use cortex_m::peripheral::NVIC;
use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, pac, peripherals, rng};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::{Error, ErrorKind, ErrorType, Read, Write};
use interrupt::InterruptExt as _;
use nrf_sdc::{
    mpsl::{mpsl_init, mpsl_run, Config as MpslConfig, LfClock},
    raw,
    sdc::{sdc_hci_get, sdc_hci_write_command, sdc_hci_write_data, sdc_init, try_sdc_hci_get, Config as SdcConfig},
    Error as SdcError,
};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    RNG => embassy_nrf::rng::InterruptHandler<peripherals::RNG>;
    SWI0_EGU0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    POWER_CLOCK => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

fn current_millis() -> u64 {
    Instant::now().as_millis()
}

#[embassy_executor::task]
async fn mpsl_task() {
    loop {
        mpsl_run().await
    }
}

fn bd_addr() -> [u8; 6] {
    unsafe {
        let ficr = &*pac::FICR::ptr();
        let high = u64::from((ficr.deviceid[1].read().bits() & 0x0000ffff) | 0x0000c000);
        let addr = high << 32 | u64::from(ficr.deviceid[0].read().bits());
        unwrap!(addr.to_le_bytes()[..6].try_into())
    }
}

#[embassy_executor::main]
async fn main(_s: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    let p = embassy_nrf::init(config);

    interrupt::RTC0.set_priority(interrupt::Priority::P0);
    interrupt::RADIO.set_priority(interrupt::Priority::P0);
    interrupt::TIMER0.set_priority(interrupt::Priority::P0);
    interrupt::POWER_CLOCK.set_priority(interrupt::Priority::P4);
    interrupt::SWI0_EGU0.set_priority(interrupt::Priority::P4);
    info!("Init mpsl");
    let config = MpslConfig {
        source: LfClock::Rc,
        rc_ctiv: 16,
        rc_temp_ctiv: 2,
        accuracy_ppm: 250,
    };

    // let config = MpslConfig {
    //     source: LfClock::Xtal,
    //     rc_ctiv: 0,
    //     rc_temp_ctiv: 0,
    //     accuracy_ppm: 250,
    // };
    mpsl_init(config, Irqs).unwrap();
    Timer::after(Duration::from_millis(10)).await;
    _s.spawn(mpsl_task()).unwrap();

    let mut rng = rng::Rng::new(p.RNG, Irqs);
    rng.set_bias_correction(true);
    let mut seed = [0u8; 32];
    rng.blocking_fill_bytes(&mut seed);

    Timer::after(Duration::from_millis(10)).await;
    let config = SdcConfig { seed };
    sdc_init(config).unwrap();

    let mut hci_buf = [0; raw::HCI_MSG_BUFFER_MAX_SIZE as usize];

    let bd_addr = bd_addr();
    let ret =
        unsafe { raw::sdc_hci_cmd_vs_zephyr_write_bd_addr(&raw::sdc_hci_cmd_vs_zephyr_write_bd_addr_t { bd_addr }) };

    let connector = SdHci::new();
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

    loop {
        Timer::after(Duration::from_millis(3000)).await;
    }
}

pub struct SdHci {
    buffer: [u8; nrf_sdc::raw::HCI_MSG_BUFFER_MAX_SIZE as usize],
    rpos: usize,
    end: usize,

    write_buffer: [u8; nrf_sdc::raw::HCI_MSG_BUFFER_MAX_SIZE as usize],
    wpos: usize,
}

impl SdHci {
    pub fn new() -> Self {
        Self {
            buffer: [0; nrf_sdc::raw::HCI_MSG_BUFFER_MAX_SIZE as usize],
            rpos: 0,
            end: 0,

            write_buffer: [0; nrf_sdc::raw::HCI_MSG_BUFFER_MAX_SIZE as usize],
            wpos: 0,
        }
    }

    fn fetch_next(&mut self) -> Result<(), SdcError> {
        if self.rpos == self.end {
            try_sdc_hci_get(&mut self.buffer)?;
            let _type = self.buffer[0];
            let len = match _type {
                0x04 => {
                    let _code = self.buffer[1];
                    let len = self.buffer[2];
                    3 + len as usize
                }
                0x02 => {
                    let _handle = &self.buffer[1..2];
                    let len = u16::from_le_bytes([self.buffer[3], self.buffer[4]]);
                    5 + len as usize
                }
                0x01 => 1,
                _ => {
                    return Err(SdcError::Other);
                }
            };
            info!("Got packet of len {}", len);
            assert!(len <= self.buffer.len());
            self.rpos = 0;
            self.end = len;
            Ok(())
        } else {
            Ok(())
        }
    }
}

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
        match self.fetch_next() {
            Ok(_) => {
                let to_copy = core::cmp::min(buf.len(), self.end - self.rpos);
                defmt::info!("Copying {} bytes", to_copy);
                buf[..to_copy].copy_from_slice(&self.buffer[self.rpos..self.rpos + to_copy]);
                self.rpos += to_copy;
                Ok(to_copy)
            }
            Err(SdcError::Again) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }
}

impl embedded_io::Write for SdHci {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        assert!(self.wpos + buf.len() <= self.write_buffer.len());
        self.write_buffer[self.wpos..self.wpos + buf.len()].copy_from_slice(buf);
        self.wpos += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        assert!(self.wpos >= 1);
        let t = self.write_buffer[0];
        match t {
            0x01 => {
                sdc_hci_write_command(&self.write_buffer[1..self.wpos])?;
            }
            0x02 => {
                sdc_hci_write_data(&self.write_buffer[1..self.wpos])?;
            }
            _ => return Err(SdcError::Other.into()),
        }
        self.wpos = 0;
        Ok(())
    }
}
