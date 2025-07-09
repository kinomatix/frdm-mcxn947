use embassy_sync::waitqueue::AtomicWaker;

const PORT_COUNT: usize = 5;

/// Initialization Logic
/// Note: GPIO port clocks are initialized in the clocks module.
pub(crate) fn init() {
    unsafe {
        // reset gpio
        crate::pac::syscon::SYSCON0::instance()
            .regs()
            .PRESETCTRL0()
            .modify(|r| {
                r.set_PORT0_RST(true);
                r.set_GPIO0_RST(true);
                r.set_PORT1_RST(true);
                r.set_GPIO1_RST(true);
                r.set_PORT2_RST(true);
                r.set_GPIO2_RST(true);
                r.set_PORT3_RST(true);
                r.set_GPIO3_RST(true);
                r.set_PORT4_RST(true);
                r.set_GPIO4_RST(true);
            });

        crate::pac::syscon::SYSCON0::instance()
            .regs()
            .PRESETCTRL0()
            .modify(|r| {
                r.set_PORT0_RST(false);
                r.set_GPIO0_RST(false);
                r.set_PORT1_RST(false);
                r.set_GPIO1_RST(false);
                r.set_PORT2_RST(false);
                r.set_GPIO2_RST(false);
                r.set_PORT3_RST(false);
                r.set_GPIO3_RST(false);
                r.set_PORT4_RST(false);
                r.set_GPIO4_RST(false);
            });

        // enable GPIO0 clock
        crate::pac::syscon::SYSCON0::instance()
            .regs()
            .AHBCLKCTRL0()
            .modify(|ahb_ctrl0| {
                ahb_ctrl0.set_GPIO0(true);
                ahb_ctrl0.set_PORT0(true);
                ahb_ctrl0.set_GPIO1(true);
                ahb_ctrl0.set_PORT1(true);
                ahb_ctrl0.set_GPIO2(true);
                ahb_ctrl0.set_PORT2(true);
                ahb_ctrl0.set_GPIO3(true);
                ahb_ctrl0.set_PORT3(true);
                ahb_ctrl0.set_GPIO4(true);
                ahb_ctrl0.set_PORT4(true);
            });
    }

    // Enable INTA
    //TODO interrupt::GPIO00.unpend();

    // SAFETY:
    //
    // At this point, all GPIO interrupts are masked. No interrupts
    // will trigger until a pin is configured as Input, which can only
    // happen after initialization of the HAL
    //TODO unsafe { interrupt::GPIO00.enable() };
}

/// AnyPin is a simple way to represent any GPIO pin
///
/// The byte contains the port and pin index in the upper and lower nibbles
pub struct AnyPin {
    pin: u8,
}

#[allow(dead_code)]
impl AnyPin {
    pub unsafe fn steal(&self) -> AnyPin {
        AnyPin { pin: self.pin }
    }

    /// Get the bank id
    #[inline]
    fn bank_id(&self) -> u8 {
        (self.pin >> 4) & 0x0F
    }

    /// Get the pin id
    #[inline]
    fn pin_id(&self) -> u8 {
        self.pin & 0x0F
    }

    /// Pin mask
    #[inline]
    fn mask(&self) -> u32 {
        1 << self.pin_id()
    }

    /// Get the pin's associated GPIO register
    #[inline]
    fn gpio(&self) -> crate::pac::gpio::GPIO {
        match self.bank_id() {
            0 => unsafe { crate::pac::gpio::GPIO0::instance().regs() },
            1 => unsafe { crate::pac::gpio::GPIO1::instance().regs() },
            2 => unsafe { crate::pac::gpio::GPIO2::instance().regs() },
            3 => unsafe { crate::pac::gpio::GPIO3::instance().regs() },
            4 => unsafe { crate::pac::gpio::GPIO4::instance().regs() },
            _ => unreachable!("invalid bank"),
        }
    }

    /// Get the pin's associated PORT register
    #[inline]
    fn port(&self) -> crate::pac::port::PORT {
        match self.bank_id() {
            0 => unsafe { crate::pac::port::PORT0::instance().regs() },
            1 => unsafe { crate::pac::port::PORT1::instance().regs() },
            2 => unsafe { crate::pac::port::PORT2::instance().regs() },
            3 => unsafe { crate::pac::port::PORT3::instance().regs() },
            4 => unsafe { crate::pac::port::PORT4::instance().regs() },
            _ => unreachable!("invalid bank"),
        }
    }

    /// Put the pin into input mode.
    ///
    /// The pull setting is left unchanged.
    #[inline]
    pub fn set_as_input(&mut self) {
        // Enable the pin as gpio in the mux
        self.port().PCR(self.pin_id() as usize).modify(|r| {
            r.set_MUX(0);
            r.set_IBE(true);
        });

        // Set the data direction to input
        self.gpio()
            .PDDR()
            .modify(|r| r.set_PDD(self.pin_id() as usize, false));
    }

    /// Put the pin into output mode.
    ///
    /// The pin level will be whatever was set before (or low by default). If you want it to begin
    /// at a specific level, call `set_high`/`set_low` on the pin first.
    #[inline]
    pub fn set_as_output(&mut self) {
        // Enable the pin as gpio in the mux
        self.port()
            .PCR(self.pin_id() as usize)
            .modify(|r| r.set_MUX(0));

        // Set the data direction to output
        self.gpio()
            .PDDR()
            .modify(|r| r.set_PDD(self.pin_id() as usize, true));
    }

    /// Set GPIO pin output.
    pub fn set(&self) {
        self.gpio().PSOR().write(|r| r.0 = self.mask());
    }

    /// Clear GPIO pin output.
    pub fn clear(&self) {
        self.gpio().PCOR().write(|r| r.0 = self.mask());
    }

    /// Toggle GPIO pin output.
    pub fn toggle(&self) {
        self.gpio().PTOR().write(|r| r.0 = self.mask());
    }

    /// Return `true` if GPIO pin is set.
    pub fn is_set(&self) -> bool {
        self.gpio().PDR(self.bank_id() as usize).read().0 != 0
    }
}

macro_rules! impl_pin {
    ($pin_name:ident, $pin_port:expr, $pin_no:expr) => {
        #[allow(dead_code)]
        pub const $pin_name: AnyPin = AnyPin {
            pin: $pin_port << 4 | $pin_no,
        };
    };
}

/// Container for pin wakers
struct PortWaker {
    offset: usize,
    wakers: &'static [AtomicWaker],
}

impl PortWaker {
    fn get_waker(&self, pin: usize) -> Option<&AtomicWaker> {
        self.wakers.get(pin - self.offset)
    }
}

macro_rules! define_port_waker {
    ($name:ident, $start:expr, $end:expr) => {
        mod $name {
            static PIN_WAKERS: [super::AtomicWaker; $end - $start + 1] =
                [const { super::AtomicWaker::new() }; $end - $start + 1];
            pub static WAKER: super::PortWaker = super::PortWaker {
                offset: $start,
                wakers: &PIN_WAKERS,
            };
        }
    };
}

// GPIO port 0
define_port_waker!(port0_waker, 0, 31);
impl_pin!(P0_0, 0, 0);
impl_pin!(P0_1, 0, 1);
impl_pin!(P0_2, 0, 2);
impl_pin!(P0_3, 0, 3);
impl_pin!(P0_4, 0, 4);
impl_pin!(P0_5, 0, 5);
impl_pin!(P0_6, 0, 6);
impl_pin!(P0_7, 0, 7);
impl_pin!(P0_8, 0, 8);
impl_pin!(P0_9, 0, 9);
impl_pin!(P0_10, 0, 10);
impl_pin!(P0_11, 0, 11);
impl_pin!(P0_12, 0, 12);
impl_pin!(P0_13, 0, 13);
impl_pin!(P0_14, 0, 14);
impl_pin!(P0_15, 0, 15);
impl_pin!(P0_16, 0, 16);
impl_pin!(P0_17, 0, 17);
impl_pin!(P0_18, 0, 18);
impl_pin!(P0_19, 0, 19);
impl_pin!(P0_20, 0, 20);
impl_pin!(P0_21, 0, 21);
impl_pin!(P0_22, 0, 22);
impl_pin!(P0_23, 0, 23);
impl_pin!(P0_24, 0, 24);
impl_pin!(P0_25, 0, 25);
impl_pin!(P0_26, 0, 26);
impl_pin!(P0_27, 0, 27);
impl_pin!(P0_28, 0, 28);
impl_pin!(P0_29, 0, 29);
impl_pin!(P0_30, 0, 30);
impl_pin!(P0_31, 0, 31);

// GPIO port 1
define_port_waker!(port1_waker, 0, 31);

// GPIO port 2
define_port_waker!(port2_waker, 0, 31);

// GPIO port 3
define_port_waker!(port3_waker, 0, 31);

// GPIO port 4
define_port_waker!(port4_waker, 0, 31);

static GPIO_WAKERS: [Option<&PortWaker>; PORT_COUNT] = [
    Some(&port0_waker::WAKER),
    Some(&port1_waker::WAKER),
    Some(&port2_waker::WAKER),
    Some(&port3_waker::WAKER),
    Some(&port4_waker::WAKER),
];

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO00() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO01() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO10() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO11() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO20() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO21() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO30() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO31() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO40() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO41() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO50() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
#[interrupt]
#[allow(non_snake_case)]
fn GPIO51() {
    irq_handler(&GPIO_WAKERS);
}

#[cfg(feature = "rt")]
fn irq_handler(port_wakers: &[Option<&PortWaker>]) {
    //TODO wake wakers
    //let reg = unsafe { crate::pac::Gpio::steal() };

    //for (port, port_waker) in port_wakers.iter().enumerate() {
    //    let Some(port_waker) = port_waker else {
    //        continue;
    //    };

    //    let stat = reg.intstata(port).read().bits();
    //    for pin in BitIter(stat) {
    //        // Clear the interrupt from this pin
    //        reg.intstata(port)
    //            .write(|w| unsafe { w.status().bits(1 << pin) });
    //        // Disable interrupt from this pin
    //        reg.intena(port)
    //            .modify(|r, w| unsafe { w.int_en().bits(r.int_en().bits() & !(1 << pin)) });

    //        let Some(waker) = port_waker.get_waker(pin as usize) else {
    //            continue;
    //        };

    //        waker.wake();
    //    }
    //}
}
