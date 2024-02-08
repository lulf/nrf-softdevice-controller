use super::raw;
use super::Error;

pub struct Config {}

/// Initialize the softdevice controller. Should only be called once!
pub fn mpsl_init(config: Config, irq: impl cortex_m::interrupt::InterruptNumber) -> Result<(), Error> {
    let ret = unsafe { raw::mpsl_init(core::ptr::null(), irq.number() as i16, Some(fault_handler)) };
    info!("[mpsl] init return value {}", ret);
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(())
}

unsafe extern "C" fn fault_handler(file: *const i8, line: u32) {
    panic!("sdc fault handler file {:?} line {}", file, line);
}
