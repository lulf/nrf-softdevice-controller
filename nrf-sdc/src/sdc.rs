use super::raw;
use super::Error;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use rand_chacha::rand_core::{RngCore as _, SeedableRng as _};

pub struct Config {
    pub seed: [u8; 32],
}

static RNG: CriticalSectionMutex<RefCell<Option<rand_chacha::ChaCha12Rng>>> =
    CriticalSectionMutex::new(RefCell::new(None));

/// Initialize the softdevice controller. Should only be called once!
pub fn sdc_init(config: Config) -> Result<(), Error> {
    RNG.lock(|rng| {
        *rng.borrow_mut() = Some(rand_chacha::ChaCha12Rng::from_seed(config.seed));
    });

    /// Initialize msp
    let ret = unsafe { raw::sdc_init(Some(fault_handler)) };
    if ret != 0 {
        return Err(ret.into());
    }

    // Register random source
    let rand_source = raw::sdc_rand_source_t {
        rand_poll: Some(rng_poll),
        rand_prio_high_get: Some(rng_prio_high),
        rand_prio_low_get: Some(rng_prio_low),
    };
    let ret = unsafe { raw::sdc_rand_source_register(&rand_source) };
    if ret != 0 {
        return Err(ret.into());
    }
    info!("[sdc] rand registered");

    let mut memory_usage = 0;
    let ret = unsafe {
        raw::sdc_cfg_set(
            ConfigTag::Default as u8,
            ConfigType::AdvCount as u8,
            &raw::sdc_cfg_t {
                adv_count: raw::sdc_cfg_role_count_t { count: 1 },
            },
        )
    };
    if ret < 0 {
        return Err(ret.into());
    }
    memory_usage += ret;
    info!("[sdc] set adv count");

    let ret = unsafe {
        raw::sdc_cfg_set(
            ConfigTag::Default as u8,
            ConfigType::PeripheralCount as u8,
            &raw::sdc_cfg_t {
                peripheral_count: raw::sdc_cfg_role_count_t { count: 1 },
            },
        )
    };
    if ret < 0 {
        return Err(ret.into());
    }
    memory_usage += ret;

    static mut BUFFER: [u8; 8192] = [0; 8192];

    let ret = unsafe { raw::sdc_enable(Some(hci_callback), BUFFER.as_mut_ptr()) };
    if ret != 0 {
        return Err(ret.into());
    }

    info!("[sdc] init done. Required memory {}", memory_usage);
    Ok(())
}

/// Initialize the softdevice controller. Should only be called once!
pub fn sdc_hci_write(data: &[u8]) -> Result<(), Error> {
    info!("[sdc] write {}", data.len());
    let ret = unsafe { raw::sdc_hci_data_put(data.as_ptr()) };
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(())
}

pub fn sdc_hci_read(data: &mut [u8]) -> Result<usize, Error> {
    let mut t: u32 = 0;
    let ret = unsafe { raw::sdc_hci_get(data.as_mut_ptr(), (&mut t) as *mut u32) };
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(data.len())
}

#[repr(u8)]
enum ConfigTag {
    Default = 0,
}

#[repr(u8)]
enum ConfigType {
    None = 0,
    CentralCount = 1,
    PeripheralCount = 2,
    Buffer = 3,
    AdvCount = 4,
}

unsafe extern "C" fn hci_callback() {
    info!("[sdc] hci event!");
}

unsafe extern "C" fn rng_prio_low(buf: *mut u8, len: u8) -> u8 {
    rng_poll(buf, len);
    len
}

unsafe extern "C" fn rng_prio_high(buf: *mut u8, len: u8) -> u8 {
    rng_poll(buf, len);
    len
}

unsafe extern "C" fn rng_poll(buf: *mut u8, len: u8) {
    RNG.lock(|rng| {
        let mut rng = rng.borrow_mut();
        let rng = rng.as_mut().unwrap();
        let buf = core::ptr::slice_from_raw_parts_mut(buf, len as usize);
        rng.fill_bytes(&mut *buf);
    })
}

unsafe extern "C" fn fault_handler(file: *const i8, line: u32) {
    panic!("sdc fault handler file {:?} line {}", file, line);
}
