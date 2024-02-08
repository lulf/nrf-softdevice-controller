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
    info!("[sdc] init return value {}", ret);
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
    Ok(())
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
