#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bos::println;
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello world{}", "!");

    bos::init();

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    bos::test_panic_handler(info)
}
