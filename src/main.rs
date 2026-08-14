#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use blog_os::{println, vga_buffer::Color, vga_buffer::enable_blink};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    enable_blink();

    // Blinking warning text (Red text, Black background, blink = true)
    println!([Color::Yellow, Color::Black, false], "WELCOME TO THE BLOG!!!");

    blog_os::init();

    println!([Color::Red, Color::Black, true], "MAIN IS FINISHED!!!");

    #[cfg(test)]
    test_main();

    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
