#![no_std]
#![no_main]

mod vga_buffer;
use vga_buffer::Color;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    vga_buffer::enable_blink();

    // Standard text
    println!("System online.");

    // Colored non-blinking text
    println!([Color::Green, Color::Black], "Status: OK");

    // Blinking warning text (Red text, Black background, blink = true)
    println!([Color::Black, Color::Magenta, true], "WARNING: CRITICAL SYSTEM ALERT!");

    loop {}
}

use core::panic::PanicInfo;

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
