use crate::system;
use arbitrary_int::prelude::*;
use core::{alloc::Layout, ptr::NonNull};
use embedded_graphics::{
    pixelcolor::{
        Rgb555, Rgb888,
        raw::{RawU16, RawU32},
    },
    prelude::*,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct RGBA8888(RawU32);

impl embedded_graphics::pixelcolor::PixelColor for RGBA8888 {
    type Raw = RawU32;
}

impl RGBA8888 {
    #[inline]
    pub const fn from_u32(c: u32) -> Self {
        Self(RawU32::new(c))
    }
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from_u32(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32)
    }
    #[inline]
    pub const fn linear(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = r as u32;
        let g = g as u32;
        let b = b as u32;
        Self::new(
            ((r * r) >> 8) as u8,
            ((g * g) >> 8) as u8,
            ((b * b) >> 8) as u8,
            a,
        )
    }
    #[inline]
    pub const fn into_u32(self) -> u32 {
        unsafe { core::mem::transmute(self.0) }
    }
    #[inline]
    pub const fn from_rgba5551(c: RGBA5551) -> Self {
        let c = c.into_u16();
        let r = ((c >> 11) & 0x1F) as u8;
        let g = ((c >> 6) & 0x1F) as u8;
        let b = ((c >> 1) & 0x1F) as u8;
        let a = if (c & 0x1) != 0 { 0xFF } else { 0 };
        Self::new(
            (r << 3) | (r >> 2),
            (g << 3) | (g >> 2),
            (b << 3) | (b >> 2),
            a,
        )
    }
    #[inline]
    const fn from_rgb888(c: Rgb888) -> Self {
        let c: u32 = unsafe { core::mem::transmute(c) };
        Self::from_u32((c << 8) | 0xFF)
    }
    #[inline]
    pub const fn r(self) -> u8 {
        (self.into_u32() >> 24) as u8
    }
    #[inline]
    pub const fn g(self) -> u8 {
        (self.into_u32() >> 16) as u8
    }
    #[inline]
    pub const fn b(self) -> u8 {
        (self.into_u32() >> 8) as u8
    }
    #[inline]
    pub const fn a(self) -> u8 {
        self.into_u32() as u8
    }
}

impl RgbColor for RGBA8888 {
    const MAX_R: u8 = 0xFF;
    const MAX_G: u8 = 0xFF;
    const MAX_B: u8 = 0xFF;
    const BLACK: Self = Self::from_rgb888(Rgb888::BLACK);
    const RED: Self = Self::from_rgb888(Rgb888::RED);
    const GREEN: Self = Self::from_rgb888(Rgb888::GREEN);
    const BLUE: Self = Self::from_rgb888(Rgb888::BLUE);
    const YELLOW: Self = Self::from_rgb888(Rgb888::YELLOW);
    const MAGENTA: Self = Self::from_rgb888(Rgb888::MAGENTA);
    const CYAN: Self = Self::from_rgb888(Rgb888::CYAN);
    const WHITE: Self = Self::from_rgb888(Rgb888::WHITE);
    #[inline]
    fn r(&self) -> u8 {
        (*self).r()
    }
    #[inline]
    fn g(&self) -> u8 {
        (*self).g()
    }
    #[inline]
    fn b(&self) -> u8 {
        (*self).b()
    }
}

impl From<u32> for RGBA8888 {
    #[inline]
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl From<RGBA8888> for u32 {
    #[inline]
    fn from(value: RGBA8888) -> Self {
        value.into_u32()
    }
}

impl From<RGBA5551> for RGBA8888 {
    #[inline]
    fn from(value: RGBA5551) -> Self {
        Self::from_rgba5551(value)
    }
}

impl From<RawU32> for RGBA8888 {
    #[inline]
    fn from(value: RawU32) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct RGBA5551(RawU16);

impl embedded_graphics::pixelcolor::PixelColor for RGBA5551 {
    type Raw = RawU16;
}

impl RGBA5551 {
    #[inline]
    pub const fn from_u16(c: u16) -> Self {
        Self(RawU16::new(c))
    }
    #[inline]
    pub const fn new(r: u5, g: u5, b: u5, a: u1) -> Self {
        let r = r.value() as u16;
        let g = g.value() as u16;
        let b = b.value() as u16;
        let a = a.value() as u16;
        Self::from_u16((r << 11) | (g << 6) | (b << 1) | a)
    }
    #[inline]
    pub const fn linear(r: u5, g: u5, b: u5, a: u1) -> Self {
        let r = r.value() as u32;
        let g = g.value() as u32;
        let b = b.value() as u32;
        let r = unsafe { u5::new_unchecked(((r * r) >> 5) as _) };
        let g = unsafe { u5::new_unchecked(((g * g) >> 5) as _) };
        let b = unsafe { u5::new_unchecked(((b * b) >> 5) as _) };
        Self::new(r, g, b, a)
    }
    #[inline]
    pub const fn into_u16(self) -> u16 {
        unsafe { core::mem::transmute(self.0) }
    }
    #[inline]
    pub const fn from_rgba8888(c: RGBA8888) -> Self {
        Self::from_u16(
            (((c.r() as u16) >> 3) << 11)
                | (((c.g() as u16) >> 3) << 6)
                | (((c.b() as u16) >> 3) << 1)
                | ((c.a() as u16) >> 7),
        )
    }
    #[inline]
    const fn from_rgb555(c: Rgb555) -> Self {
        let c: u16 = unsafe { core::mem::transmute(c) };
        Self::from_u16((c << 1) | 0x01)
    }
    #[inline]
    pub const fn r(self) -> u5 {
        unsafe { u5::new_unchecked((self.into_u16() >> 11) as u8) }
    }
    #[inline]
    pub const fn g(self) -> u5 {
        unsafe { u5::new_unchecked(((self.into_u16() >> 6) & 0x1F) as u8) }
    }
    #[inline]
    pub const fn b(self) -> u5 {
        unsafe { u5::new_unchecked(((self.into_u16() >> 1) & 0x1F) as u8) }
    }
    #[inline]
    pub const fn a(self) -> u1 {
        unsafe { u1::new_unchecked((self.into_u16() & 1) as u8) }
    }
}

impl RgbColor for RGBA5551 {
    const MAX_R: u8 = 0x1F;
    const MAX_G: u8 = 0x1F;
    const MAX_B: u8 = 0x1F;
    const BLACK: Self = Self::from_rgb555(Rgb555::BLACK);
    const RED: Self = Self::from_rgb555(Rgb555::RED);
    const GREEN: Self = Self::from_rgb555(Rgb555::GREEN);
    const BLUE: Self = Self::from_rgb555(Rgb555::BLUE);
    const YELLOW: Self = Self::from_rgb555(Rgb555::YELLOW);
    const MAGENTA: Self = Self::from_rgb555(Rgb555::MAGENTA);
    const CYAN: Self = Self::from_rgb555(Rgb555::CYAN);
    const WHITE: Self = Self::from_rgb555(Rgb555::WHITE);
    #[inline]
    fn r(&self) -> u8 {
        (*self).r().value()
    }
    #[inline]
    fn g(&self) -> u8 {
        (*self).g().value()
    }
    #[inline]
    fn b(&self) -> u8 {
        (*self).b().value()
    }
}

impl From<u16> for RGBA5551 {
    #[inline]
    fn from(value: u16) -> Self {
        Self::from_u16(value)
    }
}

impl From<RGBA5551> for u16 {
    #[inline]
    fn from(value: RGBA5551) -> Self {
        value.into_u16()
    }
}

impl From<RGBA8888> for RGBA5551 {
    #[inline]
    fn from(value: RGBA8888) -> Self {
        Self::from_rgba8888(value)
    }
}

impl From<RawU16> for RGBA5551 {
    #[inline]
    fn from(value: RawU16) -> Self {
        Self(value)
    }
}

pub struct Surface<P> {
    ptr: NonNull<P>,
    width: u16,
    height: u16,
}

impl<P> Surface<P> {
    #[inline]
    const unsafe fn layout(width: u16, height: u16, mut align: usize) -> Layout {
        let size = width as usize * height as usize * size_of::<P>();
        if align < 16 {
            align = 16;
        }
        if align < size_of::<P>() {
            align = size_of::<P>();
        }
        unsafe { Layout::from_size_align_unchecked(size, align) }
    }
    fn _new(width: u16, height: u16, align: usize) -> Self {
        let layout = unsafe { Self::layout(width, height, align) };
        let ptr = unsafe { alloc::alloc::GlobalAlloc::alloc(&system::SYSTEM, layout) };
        let Some(ptr) = NonNull::new(ptr as _) else {
            panic!("memory allocation of {} bytes failed", layout.size());
        };
        Self {
            ptr: system::uncached_addr(ptr),
            width,
            height,
        }
    }
    #[inline]
    pub fn framebuffer(width: u16, height: u16) -> Self {
        Self::_new(width, height, 0x100000)
    }
    #[inline]
    pub fn new(width: u16, height: u16) -> Self {
        Self::_new(width, height, 16)
    }
    #[inline]
    pub const fn as_ptr(&self) -> *const P {
        self.ptr.as_ptr() as _
    }
    #[inline]
    pub const fn as_mut_ptr(&mut self) -> *mut P {
        self.ptr.as_ptr()
    }
    #[inline]
    pub const fn width(&self) -> u16 {
        self.width
    }
    #[inline]
    pub const fn height(&self) -> u16 {
        self.height
    }
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width != 0 && self.height != 0
    }
    #[inline]
    pub const fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }
    #[inline]
    pub fn as_slice(&self) -> &[P] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.len()) }
    }
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [P] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }
}

impl<P> Drop for Surface<P> {
    fn drop(&mut self) {
        unsafe {
            let ptr = system::cached_addr(self.ptr).as_ptr();
            let layout = Self::layout(self.width, self.height, 16);
            alloc::alloc::GlobalAlloc::dealloc(&system::SYSTEM, ptr as _, layout);
        }
    }
}

impl<P> embedded_graphics::geometry::OriginDimensions for Surface<P> {
    #[inline]
    fn size(&self) -> embedded_graphics::geometry::Size {
        embedded_graphics::geometry::Size::new(self.width as u32, self.height as u32)
    }
}

impl<P: embedded_graphics::pixelcolor::PixelColor> embedded_graphics::draw_target::DrawTarget
    for Surface<P>
{
    type Color = P;
    type Error = core::convert::Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for embedded_graphics::Pixel(p, c) in pixels {
            if p.x >= 0 && p.x <= self.width as i32 && p.y >= 0 && p.y <= self.height as i32 {
                unsafe {
                    self.ptr
                        .as_ptr()
                        .add(p.x as usize + p.y as usize * self.width as usize)
                        .write(c)
                };
            }
        }
        Ok(())
    }
}
