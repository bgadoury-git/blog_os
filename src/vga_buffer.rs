#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        Self::new_with_blink(foreground, background, false)
    }

    pub fn new_with_blink(foreground: Color, background: Color, blink: bool) -> ColorCode {
        // Mask background to 3 bits (0..7) so it doesn't overlap bit 7
        let bg = (background as u8) & 0x07;
        let blink_bit = if blink { 0x80 } else { 0x00 };
        ColorCode(blink_bit | (bg << 4) | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

use volatile::Volatile;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::new(Color::Black, Color::Black),
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

use core::fmt;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _print_color(fg: Color, bg: Color, args: fmt::Arguments) {
    use core::fmt::Write;
    let mut writer = WRITER.lock();
    let previous_color = writer.color_code;
    writer.color_code = ColorCode::new(fg, bg);
    writer.write_fmt(args).unwrap();
    writer.color_code = previous_color;
}

#[doc(hidden)]
pub fn _print_color_blink(fg: Color, bg: Color, blink: bool, args: fmt::Arguments) {
    use core::fmt::Write;
    let mut writer = WRITER.lock();
    let previous_color = writer.color_code;
    writer.color_code = ColorCode::new_with_blink(fg, bg, blink);
    writer.write_fmt(args).unwrap();
    writer.color_code = previous_color;
}

#[macro_export]
macro_rules! print {
    // 1. With foreground, background, AND blink flag: print!([fg, bg, blink], "fmt", ...)
    ([$fg:expr, $bg:expr, $blink:expr], $($arg:tt)*) => {
        $crate::vga_buffer::_print_color_blink($fg, $bg, $blink, format_args!($($arg)*))
    };
    // 2. With foreground and background (defaults blink to false)
    ([$fg:expr, $bg:expr], $($arg:tt)*) => {
        $crate::vga_buffer::_print_color_blink($fg, $bg, false, format_args!($($arg)*))
    };
    // 3. Standard print using default WRITER color
    ($($arg:tt)*) => {
        $crate::vga_buffer::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ([$fg:expr, $bg:expr, $blink:expr]) => ($crate::print!([$fg, $bg, $blink], "\n"));
    ([$fg:expr, $bg:expr]) => ($crate::print!([$fg, $bg, false], "\n"));
    
    // Changed $arg:tt to $arg:expr and added optional trailing comma $(,)? support
    ([$fg:expr, $bg:expr, $blink:expr], $fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::print!([$fg, $bg, $blink], concat!($fmt, "\n") $(, $arg)*)
    };
    ([$fg:expr, $bg:expr], $fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::print!([$fg, $bg, false], concat!($fmt, "\n") $(, $arg)*)
    };
    ($fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::print!(concat!($fmt, "\n") $(, $arg)*)
    };
}

use core::arch::asm;

/// Enables hardware text blinking by setting Bit 3 of VGA Attribute Register 0x10.
pub fn enable_blink() {
    unsafe {
        let mut mode: u8;

        // ==========================================
        // READ PHASE
        // ==========================================
        // 1. Reset flip-flop to Index Mode
        asm!("in al, dx", in("dx") 0x3DAu16, out("al") _);
        
        // 2. Write Index 0x10 (with Bit 5 set to preserve display: 0x10 | 0x20 = 0x30)
        asm!("out dx, al", in("dx") 0x3C0u16, in("al") 0x30u8);
        
        // 3. Read current attribute mode byte
        asm!("in al, dx", in("dx") 0x3C1u16, out("al") mode);


        // ==========================================
        // MODIFY PHASE
        // ==========================================
        // 4. Set Bit 3 (0x08) to enable Blink Mode
        mode |= 0x08;


        // ==========================================
        // WRITE PHASE
        // ==========================================
        // 5. CRITICAL: Reset flip-flop back to Index Mode AGAIN before writing
        asm!("in al, dx", in("dx") 0x3DAu16, out("al") _);
        
        // 6. Write Index 0x10 again (with Bit 5 set)
        asm!("out dx, al", in("dx") 0x3C0u16, in("al") 0x30u8);
        
        // 7. Write the updated data byte to Port 0x3C0
        asm!("out dx, al", in("dx") 0x3C0u16, in("al") mode);


        // ==========================================
        // CLEANUP
        // ==========================================
        // 8. Reset flip-flop one last time and re-enable video display
        asm!("in al, dx", in("dx") 0x3DAu16, out("al") _);
        asm!("out dx, al", in("dx") 0x3C0u16, in("al") 0x20u8);
    }
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    let s = "Some test string that fits on a single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}