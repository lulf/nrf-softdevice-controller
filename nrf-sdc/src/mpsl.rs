use super::raw;
use super::Error;
use core::future::poll_fn;
use core::task::Poll;
use cortex_m::interrupt::InterruptNumber;
use embassy_nrf::interrupt;
use embassy_nrf::interrupt::typelevel::Binding;
use embassy_nrf::interrupt::typelevel::Handler;
use embassy_nrf::interrupt::typelevel::Interrupt;
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
pub fn mpsl_init<T, I>(config: Config, _irq: I) -> Result<(), Error>
where
    T: Interrupt,
    I: Binding<T, LowPrioInterruptHandler>
        + Binding<interrupt::typelevel::RADIO, HighPrioInterruptHandler>
        + Binding<interrupt::typelevel::TIMER0, HighPrioInterruptHandler>
        + Binding<interrupt::typelevel::RTC0, HighPrioInterruptHandler>
        + Binding<interrupt::typelevel::POWER_CLOCK, ClockInterruptHandler>,
{
    // Default values
    let clock_config = raw::mpsl_clock_lfclk_cfg_t {
        source: config.source.into(),
        rc_ctiv: config.rc_ctiv,
        rc_temp_ctiv: config.rc_temp_ctiv,
        accuracy_ppm: config.accuracy_ppm,
        skip_wait_lfclk_started: false,
    };

    T::unpend();
    let ret = unsafe { raw::mpsl_init(&clock_config, T::IRQ.number() as u32, Some(mpsl_assert_handler)) };
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

unsafe extern "C" fn mpsl_assert_handler(file: *const u8, line: u32) {
    let file = core::ffi::CStr::from_ptr(file as _).to_str().unwrap();
    panic!("SDC assertion failed at file {} line {}", file, line);
}

// Low priority interrupts
pub struct LowPrioInterruptHandler;
impl<T: Interrupt> Handler<T> for LowPrioInterruptHandler {
    unsafe fn on_interrupt() {
        MPSL_WAKER.wake();
    }
}

pub struct ClockInterruptHandler;
impl Handler<interrupt::typelevel::POWER_CLOCK> for ClockInterruptHandler {
    unsafe fn on_interrupt() {
        raw::MPSL_IRQ_CLOCK_Handler();
    }
}

// High priority interrupts
pub struct HighPrioInterruptHandler;
impl Handler<interrupt::typelevel::RADIO> for HighPrioInterruptHandler {
    unsafe fn on_interrupt() {
        raw::MPSL_IRQ_RADIO_Handler();
    }
}

impl Handler<interrupt::typelevel::TIMER0> for HighPrioInterruptHandler {
    unsafe fn on_interrupt() {
        raw::MPSL_IRQ_TIMER0_Handler();
    }
}

impl Handler<interrupt::typelevel::RTC0> for HighPrioInterruptHandler {
    unsafe fn on_interrupt() {
        raw::MPSL_IRQ_RTC0_Handler();
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
