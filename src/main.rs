#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use bos::println;
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use bos::allocator;
    use bos::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    println!("Hello world{}", "!");
    bos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};

    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    let ref_counted = Rc::new(vec![1, 2, 3]);
    let cloned_ref = Rc::clone(&ref_counted);
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_ref)
    );
    core::mem::drop(ref_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_ref));

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    bos::hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    bos::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    bos::test_panic_handler(info)
}
