use super::raw;
use super::Error;
use core::future::poll_fn;
use core::task::Poll;
use embassy_nrf::interrupt;
use embassy_nrf::interrupt::InterruptExt as _;
use embassy_sync::waitqueue::AtomicWaker;

pub enum LfClock {
    Rc,
    Xtal,
    Synth,
}

pub struct Config {
    pub source: LfClock,
    pub rc_ctiv: u8,
    pub rc_temp_ctiv: u8,
    pub accuracy_ppm: u16,
}

impl Into<u8> for LfClock {
    fn into(self) -> u8 {
        match self {
            Self::Rc => 0,
            Self::Xtal => 1,
            Self::Synth => 2,
        }
    }
}

/// Initialize the softdevice controller. Should only be called once!
pub fn mpsl_init<T: interrupt::typelevel::Interrupt>(
    config: Config,
    irq: impl interrupt::typelevel::Binding<T, InterruptHandler>,
) -> Result<(), Error> {
    // Default values
    let clock_config = raw::mpsl_clock_lfclk_cfg_t {
        source: config.source.into(),
        rc_ctiv: config.rc_ctiv,
        rc_temp_ctiv: config.rc_temp_ctiv,
        accuracy_ppm: config.accuracy_ppm,
        skip_wait_lfclk_started: false,
    };

    let ret = unsafe { raw::mpsl_init(&clock_config, T::IRQ as i16, Some(mpsl_assert_handler)) };
    info!("Init done: {}", ret);
    if ret != 0 {
        return Err(ret.into());
    }

    let ret = unsafe { raw::mpsl_clock_hfclk_request(Some(hfclk_callback)) };
    info!("Hfclk request done: {}", ret);
    if ret != 0 {
        return Err(ret.into());
    }
    Ok(())
}

unsafe extern "C" fn hfclk_callback() {
    info!("HFCLK started");
}

unsafe extern "C" fn mpsl_assert_handler(file: *const i8, line: u32) {
    let file = core::ffi::CStr::from_ptr(file as _).to_str().unwrap();
    panic!("SDC assertion failed at file {} line {}", file, line);
}

pub struct InterruptHandler;

impl<T: interrupt::typelevel::Interrupt> interrupt::typelevel::Handler<T> for InterruptHandler {
    unsafe fn on_interrupt() {
        MPSL_WAKER.wake();
    }
}

static MPSL_WAKER: AtomicWaker = AtomicWaker::new();
pub async fn mpsl_run() {
    poll_fn(|cx| {
        MPSL_WAKER.register(cx.waker());
        unsafe { raw::mpsl_low_priority_process() }
        Poll::Pending
    })
    .await
}
