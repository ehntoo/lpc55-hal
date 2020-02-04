#![no_main]
#![no_std]

extern crate panic_semihosting;  // 4004 bytes
// extern crate panic_halt; // 672 bytes

use cortex_m_rt::entry;
use cortex_m_semihosting::dbg;
use cortex_m_semihosting::heprintln;

use lpc55_hal as hal;
use hal::prelude::*;
use hal::{
    drivers::{
        Pins,
    }
};

fn delay(count: u32){
    for i in 0 .. (1_000 * count) {

    }
}

fn read_sample(adc: & hal::raw::ADC0) -> u16{
    adc.swtrig.write(|w| unsafe {w.bits(1)});
    while adc.fctrl[0].read().fcount().bits() == 0 {
    }
    let result = adc.resfifo[0].read().bits();
    assert!( (result & 0x80000000) != 0 );
    let sample = (result & 0xffff) as u16;
    sample
}

fn autocal (adc: & hal::raw::ADC0) {
    // Calibration + offset trimming
    adc.ofstrim.write(|w| unsafe {
        w.ofstrim_a().bits(10)
        .ofstrim_b().bits(10)
    });

    // Request calibration
    adc.ctrl.modify(|_,w| {w.cal_req().set_bit()});

    // wait for auto-cal to be ready.
    while (!adc.gcc[0].read().rdy().bits()) || (!adc.gcc[1].read().rdy().bits()) {
    }

    let gain_a = adc.gcc[0].read().gain_cal().bits();
    let gain_b = adc.gcc[1].read().gain_cal().bits();

    let gcr_a = (((gain_a as u32) << 16) / (0x1FFFFu32 - gain_a as u32 )) as u16;
    let gcr_b = (((gain_b as u32) << 16) / (0x1FFFFu32 - gain_b as u32 )) as u16;

    adc.gcr[0].write(|w| unsafe {w.gcalr().bits(gcr_a)});
    adc.gcr[1].write(|w| unsafe {w.gcalr().bits(gcr_b)});

    adc.gcr[0].write(|w| {w.rdy().set_bit()});
    adc.gcr[1].write(|w| {w.rdy().set_bit()});

    while !adc.stat.read().cal_rdy().bits() {
    }
}

#[entry]
fn main() -> ! {

    heprintln!("Hello ADC").unwrap();

    // Get pointer to all device peripherals.
    let mut hal = hal::new();

    let _clocks = hal::ClockRequirements::default()
        .system_frequency(12.mhz())
        .configure(&mut hal.anactrl, &mut hal.pmc, &mut hal.syscon)
        .unwrap();

    let mut gpio = hal.gpio.enabled(&mut hal.syscon);

    let adc = hal::Adc::from(hal.ADC0).enabled(&mut hal.pmc, &mut hal.syscon);
    // let adc = hal::Adc::from(dp.ADC0).enabled(&mut syscon);

    let mut iocon = hal.iocon.enabled(&mut hal.syscon);
    let pins = Pins::take().unwrap();
    let _but1 = pins.pio1_9.into_analog_input(&mut iocon, &mut gpio);
    let _but2 = pins.pio0_31.into_analog_input(&mut iocon, &mut gpio);
    let _but3 = pins.pio0_15.into_analog_input(&mut iocon, &mut gpio);

    let mut charge_pin = pins.pio1_16.into_gpio_pin(&mut iocon, &mut gpio).into_output_high();
    charge_pin.set_low().unwrap();

    let adc = adc.release();

    adc.ctrl.write(|w| {
        w.dozen().set_bit()
        .cal_avgs().cal_avgs_7()
    });

    adc.cfg.write(|w| unsafe {
        w.pwren().set_bit()
        .pudly().bits(0x80)
        .refsel().refsel_1()
        .pwrsel().pwrsel_1()
        .tprictrl().bits(0)
    });


    // No pause for now, but could be interesting
    adc.pause.write(|w| unsafe {w.bits(0)});

    // Set 0 for watermark
    adc.fctrl[0].write(|w| unsafe{w.fwmark().bits(0)});
    adc.fctrl[1].write(|w| unsafe{w.fwmark().bits(0)});


    // turn on!
    adc.ctrl.modify(|_, w| {w.adcen().set_bit()});

    heprintln!("Auto calibrating..").unwrap();
    autocal(& adc);

    // channel 12 (button 1), single ended A, high res
    adc.cmdl1.write(|w| unsafe {  w.adch().bits(3)
                                .ctype().ctype_0()  
                                .mode().mode_0()
                                } );

    adc.cmdh1.write(|w| unsafe { w.avgs().avgs_5()      // average 2^5 samples
                                .cmpen().bits(0)        // disable compare
                                .loop_().bits(0)         // no loop
                                .next().bits(0)         // no next command
                            } );

    adc.tctrl[0].write(|w| unsafe {
        w.hten().set_bit()
        .fifo_sel_a().fifo_sel_a_0()
        .fifo_sel_b().fifo_sel_b_0()
        .tcmd().bits(1)
    });

    heprintln!("ADC CTRL. {:02X}", adc.ctrl.read().bits()).unwrap();
    heprintln!("ADC  CFG. {:02X}", adc.cfg.read().bits()).unwrap();
    heprintln!("ADC stat: {:02X}", adc.stat.read().bits()).unwrap();

    // SW trigger the trigger event 0
    adc.swtrig.write(|w| unsafe {w.bits(1)});

    dbg!("triggered");

    let count0 = adc.fctrl[0].read().fcount().bits();

    heprintln!("FIFO0 conversions {}", count0).unwrap();

    let result = adc.resfifo[0].read().bits();
    let valid = result & 0x80000000;
    let sample = (result & 0xffff) as u16;
   
    if valid != 0 {
        heprintln!("sample = {:02x}", sample).unwrap();
    } else {
        heprintln!("No result from ADC!").unwrap();
    }


    heprintln!("looping").unwrap();
    let mut samples = [0u16; 32];
    loop {
        charge_pin.set_low().unwrap();
        delay(5);
        charge_pin.set_high().unwrap();
        for i in 0..samples.len(){
            samples[i] = read_sample(&adc);
        }
        dbg!(samples);
        // heprintln!("samples = {}", sample).unwrap();
    }
}