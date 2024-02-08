use super::raw;
use super::Error;

pub struct Config {}

/// Initialize the softdevice controller. Should only be called once!
pub fn mpsl_init(config: Config, irq: impl cortex_m::interrupt::InterruptNumber) -> Result<(), Error> {
    let clock_config = raw::mpsl_clock_lfclk_cfg_t {
        source: 0,
        rc_ctiv: 1,
        rc_temp_ctiv: 0,
        accuracy_ppm: 250,
        skip_wait_lfclk_started: false,
    };

    let ret = unsafe { raw::mpsl_init(&clock_config, irq.number() as i16, Some(fault_handler)) };
    info!("[mpsl] init return value {}", ret);
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(())
}

unsafe extern "C" fn fault_handler(file: *const i8, line: u32) {
    info!("FAUT");
    panic!("sdc fault handler file {:?} line {}", file, line);
}
