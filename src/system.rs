use core::{
    ffi::{c_char, c_int, c_uint, c_void},
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

core::arch::global_asm!(include_str!("kernel.S"), main = sym crate::main);

const STACK_SIZE: usize = 0x10000;
static HEAP_END: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());

pub type PhysAddr = u32;

#[inline]
pub fn physical_addr<T>(addr: NonNull<T>) -> PhysAddr {
    (addr.as_ptr() as u32) & !0xE0000000
}

#[inline]
pub const fn virtual_cached_addr<T>(addr: PhysAddr) -> NonNull<T> {
    unsafe { NonNull::new_unchecked((addr | 0x80000000) as i32 as isize as _) }
}

#[inline]
pub const fn virtual_uncached_addr<T>(addr: PhysAddr) -> NonNull<T> {
    unsafe { NonNull::new_unchecked((addr | 0xA0000000) as i32 as isize as _) }
}

#[inline]
pub fn uncached_addr<T>(addr: NonNull<T>) -> NonNull<T> {
    addr.map_addr(|p| p | 0x2000_0000)
}

#[inline]
pub unsafe fn cached_addr<T>(addr: NonNull<T>) -> NonNull<T> {
    addr.map_addr(|p| unsafe { core::num::NonZero::new_unchecked(p.get() & !0x2000_0000) })
}

pub const INDEX_INVALIDATE_I: u8 = 0;
pub const INDEX_WRITEBACK_INVALIDATE_D: u8 = 1;
pub const INDEX_LOAD_TAG_I: u8 = 4;
pub const INDEX_LOAD_TAG_D: u8 = 5;
pub const INDEX_STORE_TAG_I: u8 = 8;
pub const INDEX_STORE_TAG_D: u8 = 9;
pub const CREATE_DIRTY_EXCLUSIVE_D: u8 = 13;
pub const HIT_INVALIDATE_I: u8 = 16;
pub const HIT_INVALIDATE_D: u8 = 17;
pub const CACHE_FILL_I: u8 = 20;
pub const HIT_WRITEBACK_INVALIDATE_D: u8 = 21;
pub const HIT_WRITEBACK_I: u8 = 24;
pub const HIT_WRITEBACK_D: u8 = 25;

#[inline(always)]
unsafe fn _cache<const OP: u8, const OFFSET: i16>(ptr: isize) {
    unsafe {
        core::arch::asm! {
            ".set noat
            cache {op}, {offset} ({ptr})",
            ptr = in(reg) ptr,
            offset = const OFFSET,
            op = const OP,
        }
    }
}

#[inline(always)]
pub unsafe fn cache<T, const OP: u8, const OFFSET: i16>(data: *const T) {
    unsafe { _cache::<OP, OFFSET>(data as isize) }
}

#[inline(always)]
pub unsafe fn cache_mut<T, const OP: u8, const OFFSET: i16>(data: *mut T) {
    unsafe { _cache::<OP, OFFSET>(data as isize) }
}

#[inline(always)]
fn data_cache_loop<T, const OP: u8>(data: &[T]) {
    let size = size_of_val(data);
    if size >= 512 * 16 {
        data_cache_writeback_invalidate_all();
    } else {
        let mut ptr = data.as_ptr() as isize;
        let end = ptr + size as isize;
        ptr &= !15;
        while ptr < end {
            ptr += 16;
            unsafe { _cache::<OP, -16>(ptr) };
        }
    }
}

#[inline]
pub fn data_cache_hit_invalidate<T>(data: &[T]) {
    data_cache_loop::<_, HIT_INVALIDATE_D>(data);
}

#[inline]
pub fn data_cache_hit_writeback<T>(data: &[T]) {
    data_cache_loop::<_, HIT_WRITEBACK_D>(data);
}

#[inline]
pub fn data_cache_hit_writeback_invalidate<T>(data: &[T]) {
    data_cache_loop::<_, HIT_WRITEBACK_INVALIDATE_D>(data);
}

#[inline]
pub fn data_cache_index_writeback_invalidate<T>(data: &[T]) {
    data_cache_loop::<_, INDEX_WRITEBACK_INVALIDATE_D>(data);
}

#[inline]
pub fn data_cache_writeback_invalidate_all() {
    let mut i = 512 * 16;
    loop {
        i -= 16 * 4;
        unsafe { _cache::<INDEX_WRITEBACK_INVALIDATE_D, 0>(i) };
        unsafe { _cache::<INDEX_WRITEBACK_INVALIDATE_D, 16>(i) };
        unsafe { _cache::<INDEX_WRITEBACK_INVALIDATE_D, 32>(i) };
        unsafe { _cache::<INDEX_WRITEBACK_INVALIDATE_D, 48>(i) };
        if i == 0 {
            break;
        }
    }
}

#[inline]
pub fn inst_cache_hit_writeback<T>(data: &[T]) {
    let size = size_of_val(data);
    let mut ptr = data.as_ptr() as isize;
    let end = ptr + size as isize;
    ptr &= !31;
    while ptr < end {
        ptr += 32;
        unsafe { _cache::<HIT_WRITEBACK_I, -32>(ptr) };
    }
}

#[inline]
pub fn inst_cache_invalidate_all() {
    let mut i = 512 * 32;
    loop {
        i -= 32 * 4;
        unsafe { _cache::<INDEX_INVALIDATE_I, 0>(i) };
        unsafe { _cache::<INDEX_INVALIDATE_I, 32>(i) };
        unsafe { _cache::<INDEX_INVALIDATE_I, 64>(i) };
        unsafe { _cache::<INDEX_INVALIDATE_I, 96>(i) };
        if i == 0 {
            break;
        }
    }
}

#[inline]
pub fn mem_size() -> u32 {
    unsafe { _boot_memsize }
}

#[inline]
pub fn tv_type() -> u8 {
    unsafe { _boot_tvtype }
}

#[inline]
pub fn console_type() -> u8 {
    unsafe { _boot_consoletype }
}

const MIN_ALIGN: usize = size_of::<*const ()>() * 2;

pub struct SystemAlloc;

#[global_allocator]
pub static SYSTEM: SystemAlloc = SystemAlloc;

unsafe impl core::alloc::GlobalAlloc for SystemAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let (Ok(align), Ok(size)) = (<_>::try_from(layout.align()), <_>::try_from(layout.size()))
        else {
            return core::ptr::null_mut();
        };
        unsafe { aligned_alloc(align, size) as *mut u8 }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        let (Ok(align), Ok(size)) = (<_>::try_from(layout.align()), <_>::try_from(layout.size()))
        else {
            return core::ptr::null_mut();
        };
        if align <= MIN_ALIGN as _ && align <= size {
            unsafe { calloc(size, 1) as *mut u8 }
        } else {
            let ptr = unsafe { aligned_alloc(align, size) } as *mut u8;
            if !ptr.is_null() {
                unsafe { core::ptr::write_bytes(ptr, 0, size as usize) };
            }
            ptr
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        unsafe { free(ptr as _) }
    }

    #[inline]
    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        let (Ok(align), Ok(new_size)) = (<_>::try_from(layout.align()), <_>::try_from(new_size))
        else {
            return core::ptr::null_mut();
        };
        if align <= MIN_ALIGN as _ && align <= new_size {
            unsafe { realloc(ptr as _, new_size) as *mut u8 }
        } else {
            let new_ptr = unsafe { aligned_alloc(align, new_size) } as *mut u8;
            if !new_ptr.is_null() {
                unsafe {
                    core::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size as _));
                    self.dealloc(ptr, layout);
                }
            }
            new_ptr
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    crate::println!("{_info}");
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn sbrk(incr: c_int) -> *mut c_void {
    let mut end = HEAP_END.load(Ordering::Relaxed);
    let heap_size = mem_size() as usize - STACK_SIZE;
    let start = unsafe { __bss_end.as_ptr() as *mut u8 };
    if end.is_null() {
        end = start;
    }
    let heap_size =
        heap_size - (start as isize).wrapping_sub(0x8000_0000_u32 as i32 as isize) as usize;
    let newend = unsafe { end.offset(incr as isize) };
    if (newend as isize).wrapping_sub(start as isize) as usize > heap_size {
        return core::ptr::null_mut();
    }
    HEAP_END.store(newend, Ordering::Relaxed);
    end as *mut c_void
}

#[unsafe(no_mangle)]
extern "C" fn kill(_pid: c_int, _sig: c_int) -> c_int {
    0
}

#[unsafe(no_mangle)]
extern "C" fn getpid() -> c_int {
    0
}

unsafe extern "C" {
    static __bss_end: [c_char; 0usize];
    static _boot_memsize: u32;
    static _boot_tvtype: u8;
    static _boot_consoletype: u8;
    fn _start();
    fn free(_: *mut c_void);
    fn aligned_alloc(_: c_uint, _: c_uint) -> *mut c_void;
    fn realloc(_: *mut c_void, _: c_uint) -> *mut c_void;
    fn calloc(_: c_uint, _: c_uint) -> *mut c_void;
}
