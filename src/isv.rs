use core::{
    fmt::Write,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use n64_pac::pi::PeripheralInterface;

const ISV_REGS: NonNull<u32> = unsafe { NonNull::new_unchecked(0xB3FF0000 as *mut u32) };
const ISV_BUFFER: NonNull<u32> = unsafe { NonNull::new_unchecked(0xB3FF0020 as *mut u32) };
const ISV_TOKEN_REG: usize = 0;
const ISV_READ_REG: usize = 1;
const ISV_WRITE_REG: usize = 5;
const ISV_BUFLEN: usize = 0x10000 - 0x20;
const ISV_MAGIC: u32 = 0x49533634;

#[inline(always)]
fn pi_wait(pi: &PeripheralInterface) {
    loop {
        let status = unsafe { pi.status.read().read };
        if !status.dma_busy() && !status.io_busy() {
            break;
        }
    }
}

static ISV_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if crate::system::console_type() != 0 {
        return;
    }
    let isv = ISV_REGS;
    let pi = unsafe { PeripheralInterface::new() };
    unsafe {
        isv.add(ISV_TOKEN_REG).write_volatile(0);
        pi_wait(&pi);
        if isv.add(ISV_TOKEN_REG).read_volatile() == 0 {
            isv.add(ISV_READ_REG).write_volatile(0);
            pi_wait(&pi);
            isv.add(ISV_WRITE_REG).write_volatile(0);
            pi_wait(&pi);
            isv.add(ISV_TOKEN_REG).write_volatile(ISV_MAGIC);
            pi_wait(&pi);
            if isv.add(ISV_TOKEN_REG).read_volatile() == ISV_MAGIC {
                ISV_ENABLED.store(true, Ordering::Relaxed);
            }
        }
    }
}

pub struct Writer;

impl Write for Writer {
    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        put(s.as_bytes());
        Ok(())
    }
}

#[inline]
pub fn write(args: core::fmt::Arguments) {
    Writer.write_fmt(args).unwrap();
}

pub fn put(s: &[u8]) {
    if !ISV_ENABLED.load(Ordering::Relaxed) || s.is_empty() {
        return;
    }
    crate::system::data_cache_hit_writeback(s);
    let isv = ISV_REGS;
    let pi = unsafe { n64_pac::pi::PeripheralInterface::new() };
    pi_wait(&pi);
    let read = s.as_ptr() as u32 & 7;
    for chunk in s.chunks(ISV_BUFLEN - 8) {
        unsafe {
            let write = isv.add(ISV_WRITE_REG).read_volatile();
            while write != isv.add(ISV_READ_REG).read_volatile() {}
            isv.add(ISV_TOKEN_REG).write_volatile(0);
        }
        let write = chunk.len() as u32 + read;
        let len = ((write + 1) & !1) - 1;
        pi_wait(&pi);
        pi.dram_addr.write(chunk.as_ptr() as u32 - read);
        pi.cart_addr.write(crate::system::physical_addr(ISV_BUFFER));
        pi.rd_len.write(len);
        pi_wait(&pi);
        unsafe {
            isv.add(ISV_READ_REG).write_volatile(read);
            pi_wait(&pi);
            isv.add(ISV_WRITE_REG).write_volatile(write);
            pi_wait(&pi);
            isv.add(ISV_TOKEN_REG).write_volatile(ISV_MAGIC);
            pi_wait(&pi);
        }
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::isv::write(::core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", ::core::format_args!($($arg)*)));
}
