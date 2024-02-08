#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, interrupt, pac::Interrupt::SWI2_EGU2, peripherals, rng};
use {defmt_rtt as _, panic_probe as _};

use bleps::{
    ad_structure::{create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE},
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    gatt, Addr, Ble, HciConnector,
};
use embedded_io_async::{Error, ErrorType, Read, Write};
use nrf_softdevice_controller::{
    mpsl::{mpsl_init, Config as MpslConfig},
    raw,
    sdc::{sdc_init, Config as SdcConfig},
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

    info!("Hello!");
    //    loop {
    //        let connector = BleConnector::new(&mut serial);
    //        let hci = HciConnector::new(connector, current_millis);
    //        let mut ble = Ble::new(&hci);
    //
    //        println!("{:?}", ble.init());
    //
    //        let local_addr = Addr::from_le_bytes(false, ble.cmd_read_br_addr().unwrap());
    //
    //        println!("{:?}", ble.cmd_set_le_advertising_parameters());
    //        println!(
    //            "{:?}",
    //            ble.cmd_set_le_advertising_data(
    //                create_advertising_data(&[
    //                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
    //                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
    //                    AdStructure::CompleteLocalName("BLEPS"),
    //                ])
    //                .unwrap()
    //            )
    //        );
    //        println!("{:?}", ble.cmd_set_le_advertise_enable(true));
    //
    //        println!("started advertising");
    //
    //        let val = Arc::new(Mutex::new(Vec::from(
    //            &b"Hello BLE! Hello BLE! 01234567890123456789 ABCDEFG abcdefg"[..],
    //        )));
    //
    //        let mut rf = {
    //            let val = val.clone();
    //            move |offset: usize, data: &mut [u8]| {
    //                let val = val.lock().unwrap();
    //                let off = offset as usize;
    //                if off < val.len() {
    //                    let len = data.len().min(val.len() - off);
    //                    data[..len].copy_from_slice(&val[off..off + len]);
    //                    println!("SEND: Offset {}, data {:x?}", offset, &data[..len]);
    //                    len
    //                } else {
    //                    0
    //                }
    //            }
    //        };
    //        let mut wf = {
    //            let val = val.clone();
    //            move |offset: usize, data: &[u8]| {
    //                println!("RECEIVED: Offset {}, data {:x?}", offset, data);
    //                let mut val = val.lock().unwrap();
    //                let off = offset as usize;
    //                if off < val.len() {
    //                    let len = val.len() - off;
    //                    let olen = data.len().min(len);
    //                    val[off..off + olen].copy_from_slice(&data[..olen]);
    //                    if data.len() > len {
    //                        val.extend_from_slice(&data[olen..]);
    //                    }
    //                }
    //            }
    //        };
    //
    //        let mut wf2 = |offset: usize, data: &[u8]| {
    //            println!("RECEIVED2: Offset {}, data {:x?}", offset, data);
    //        };
    //
    //        let mut rf3 = |_offset: usize, data: &mut [u8]| {
    //            data[..5].copy_from_slice(&b"Hola!"[..]);
    //            5
    //        };
    //        let mut wf3 = |offset: usize, data: &[u8]| {
    //            println!("RECEIVED3: Offset {}, data {:x?}", offset, data);
    //        };
    //
    //        gatt!([service {
    //            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
    //            characteristics: [
    //                characteristic {
    //                    uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
    //                    read: rf,
    //                    write: wf,
    //                },
    //                characteristic {
    //                    uuid: "957312e0-2354-11eb-9f10-fbc30a62cf38",
    //                    write: wf2,
    //                },
    //                characteristic {
    //                    name: "my_characteristic",
    //                    uuid: "987312e0-2354-11eb-9f10-fbc30a62cf38",
    //                    notify: true,
    //                    read: rf3,
    //                    write: wf3,
    //                },
    //            ],
    //        },]);
    //
    //        let mut rng = OsRng::default();
    //        let mut srv = AttributeServer::new_with_ltk(
    //            &mut ble,
    //            &mut gatt_attributes,
    //            local_addr,
    //            ltk,
    //            &mut rng,
    //        );
    //
    //        let mut pin_callback = |pin: u32| {
    //            println!("PIN is {pin}");
    //        };
    //
    //        srv.set_pin_callback(Some(&mut pin_callback));
    //
    //        let mut response = [b'H', b'e', b'l', b'l', b'o', b'0'];
    //
    //        loop {
    //            let mut notification = None;
    //
    //            if let Ok(true) = crossterm::event::poll(Duration::from_micros(1)) {
    //                let event = crossterm::event::read().unwrap();
    //                match event {
    //                    crossterm::event::Event::Key(key_event) => match key_event.code {
    //                        crossterm::event::KeyCode::Char('c')
    //                            if key_event.modifiers == crossterm::event::KeyModifiers::CONTROL =>
    //                        {
    //                            exit(0);
    //                        }
    //                        crossterm::event::KeyCode::Char('q') => {
    //                            exit(0);
    //                        }
    //                        crossterm::event::KeyCode::Char('n') => {
    //                            println!("notify if enabled");
    //                            let mut cccd = [0u8; 1];
    //                            if let Some(1) = srv.get_characteristic_value(
    //                                my_characteristic_notify_enable_handle,
    //                                0,
    //                                &mut cccd,
    //                            ) {
    //                                // if notifications enabled
    //                                if cccd[0] == 1 {
    //                                    response[5] = b'0' + ((response[5] + 1) % 10);
    //                                    notification = Some(NotificationData::new(
    //                                        my_characteristic_handle,
    //                                        &response[..],
    //                                    ));
    //                                }
    //                            }
    //                        }
    //                        crossterm::event::KeyCode::Char('x') => {
    //                            srv.disconnect(0x13).unwrap();
    //                        }
    //                        _ => (),
    //                    },
    //                    _ => (),
    //                }
    //            }
    //
    //            match srv.do_work_with_notification(notification) {
    //                Ok(res) => {
    //                    if let WorkResult::GotDisconnected = res {
    //                        println!("Received disconnect");
    //                        break;
    //                    }
    //                }
    //                Err(err) => {
    //                    println!("{:x?}", err);
    //                }
    //            }
    //        }
    //
    //        ltk = srv.get_ltk();
    //    }
}

pub struct SdControllerHci {}
