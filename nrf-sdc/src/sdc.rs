use super::raw;
use super::Error;
use core::cell::RefCell;
use core::future::poll_fn;
use core::task::Poll;
use embassy_sync::blocking_mutex::CriticalSectionMutex;
use embassy_sync::waitqueue::AtomicWaker;
use rand_chacha::rand_core::{RngCore as _, SeedableRng as _};

pub struct Config {
    pub seed: [u8; 32],
}

static RNG: CriticalSectionMutex<RefCell<Option<rand_chacha::ChaCha12Rng>>> =
    CriticalSectionMutex::new(RefCell::new(None));

const SDC_MEM_SIZE: usize = 65536;
static mut SDC_MEM: [u8; SDC_MEM_SIZE] = [0; SDC_MEM_SIZE];

/// Initialize the softdevice controller. Should only be called once!
pub fn sdc_init(config: Config) -> Result<(), Error> {
    RNG.lock(|rng| {
        *rng.borrow_mut() = Some(rand_chacha::ChaCha12Rng::from_seed(config.seed));
    });

    // Initialize msp
    let ret = unsafe { raw::sdc_init(Some(sdc_assert_handler)) };
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

    let ret = unsafe { raw::sdc_support_adv() };
    if ret != 0 {
        return Err(ret.into());
    }

    let ret = unsafe { raw::sdc_support_peripheral() };
    if ret != 0 {
        return Err(ret.into());
    }

    let ret = unsafe { raw::sdc_support_central() };
    if ret != 0 {
        return Err(ret.into());
    }

    let ret = unsafe {
        raw::sdc_cfg_set(
            raw::SDC_DEFAULT_RESOURCE_CFG_TAG as u8,
            raw::sdc_cfg_type_SDC_CFG_TYPE_ADV_COUNT as u8,
            &raw::sdc_cfg_t {
                adv_count: raw::sdc_cfg_role_count_t { count: 1 },
            },
        )
    };
    if ret < 0 {
        return Err(ret.into());
    }

    let ret = unsafe {
        raw::sdc_cfg_set(
            raw::SDC_DEFAULT_RESOURCE_CFG_TAG as u8,
            raw::sdc_cfg_type_SDC_CFG_TYPE_PERIPHERAL_COUNT as u8,
            &raw::sdc_cfg_t {
                peripheral_count: raw::sdc_cfg_role_count_t { count: 1 },
            },
        )
    };
    if ret < 0 {
        return Err(ret.into());
    }

    let ret = unsafe {
        raw::sdc_cfg_set(
            raw::SDC_DEFAULT_RESOURCE_CFG_TAG as u8,
            raw::sdc_cfg_type_SDC_CFG_TYPE_CENTRAL_COUNT as u8,
            &raw::sdc_cfg_t {
                central_count: raw::sdc_cfg_role_count_t { count: 1 },
            },
        )
    };
    if ret < 0 {
        return Err(ret.into());
    }

    let wanted_memory = unsafe {
        raw::sdc_cfg_set(
            raw::SDC_DEFAULT_RESOURCE_CFG_TAG as u8,
            raw::sdc_cfg_type_SDC_CFG_TYPE_NONE as u8,
            core::ptr::null(),
        )
    };
    if wanted_memory < 0 {
        return Err(wanted_memory.into());
    }
    assert!(wanted_memory as usize <= SDC_MEM_SIZE);
    info!("[sdc] enable (mem {})", wanted_memory);

    let ret = unsafe { raw::sdc_enable(Some(sdc_callback), SDC_MEM.as_mut_ptr()) };
    if ret != 0 {
        return Err(ret.into());
    }

    info!("[sdc] init done");
    Ok(())
}

pub fn sdc_hci_write_data(data: &[u8]) -> Result<(), Error> {
    info!("[sdc] write {}", data.len());
    let ret = unsafe { raw::sdc_hci_data_put(data.as_ptr()) };
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(())
}

pub fn sdc_hci_write_command(data: &[u8]) -> Result<(), Error> {
    info!("[sdc] write {}", data.len());
    let ret = unsafe { raw::sdc_hci_cmd_put(data.as_ptr()) };
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(())
}

pub fn try_sdc_hci_get(data: &mut [u8]) -> Result<usize, Error> {
    assert!(data.len() <= raw::HCI_MSG_BUFFER_MAX_SIZE as usize);
    let mut msg_type: raw::sdc_hci_msg_type_t = 0;
    let ret = unsafe { raw::sdc_hci_get(data[1..].as_mut_ptr(), &mut msg_type) };
    if ret != 0 {
        return Err(ret.into());
    }
    info!("Received packet type {}", msg_type);
    data[0] = msg_type as u8;
    Ok(data.len())
}

pub async fn sdc_hci_get(data: &mut [u8]) -> Result<usize, Error> {
    poll_fn(|ctx| match try_sdc_hci_get(data) {
        Err(Error::Again) => {
            SDC_WAKER.register(ctx.waker());
            Poll::Pending
        }
        res => Poll::Ready(res),
    })
    .await
}

static SDC_WAKER: AtomicWaker = AtomicWaker::new();
unsafe extern "C" fn sdc_callback() {
    info!("[sdc] hci event!");
    SDC_WAKER.wake();
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

unsafe extern "C" fn sdc_assert_handler(file: *const u8, line: u32) {
    let file = core::ffi::CStr::from_ptr(file as _).to_str().unwrap();
    panic!("SDC assertion failed at file {} line {}", file, line);
}
