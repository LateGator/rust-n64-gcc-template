#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

use n64_pac::vi::VideoInterface;

extern crate alloc;
#[macro_use]
pub mod isv;
pub mod gfx;
pub mod system;

#[inline(never)]
pub fn main() {
    isv::init();
    isv::put(b"Hello N64\n");

    let vi = unsafe { VideoInterface::new() };
    let mut fb = gfx::Surface::<gfx::RGBA5551>::framebuffer(320, 240);
    render(&mut fb);
    vid_setup_16bpp(&vi, &fb);
    loop {
        wait_vblank(&vi);
    }
}

fn render(fb: &mut gfx::Surface<gfx::RGBA5551>) {
    use embedded_graphics::{mono_font::*, prelude::*, text::*};

    const STYLE: MonoTextStyle<'static, gfx::RGBA5551> =
        MonoTextStyle::new(&profont::PROFONT_7_POINT, gfx::RGBA5551::WHITE);

    let _ = fb.clear(gfx::RGBA5551::BLUE);
    let _ = Text::new("Hello N64", Point::new(32, 32), STYLE).draw(fb);
}

fn vid_setup_16bpp(vi: &VideoInterface, fb: &gfx::Surface<gfx::RGBA5551>) {
    use n64_pac::vi::{
        AntiAliasMode, BurstReg, ColorDepth, CtrlReg, HSyncLeapReg, HSyncReg, HVideoReg, VBurstReg,
        VVideoReg, XScaleReg, YScaleReg,
    };

    static VI_REGS: [[u32; 7]; 3] = [
        [
            0x4233A, 0x271, 0x150C69, 0xC6F0C6E, 0x800300, 0x2D026D, 0x9026B,
        ],
        [
            0x3E52239, 0x20D, 0xC15, 0xC150C15, 0x6C02EC, 0x230203, 0xE0204,
        ],
        [
            0x651E39, 0x20D, 0x40C11, 0xC190C1A, 0x6C02EC, 0x2501FF, 0xE0204,
        ],
    ];

    let tv = match system::tv_type() {
        tv @ 0..=2 => tv,
        _ => 0,
    };
    let regs = &VI_REGS[tv as usize];
    let width = fb.width();
    let height = fb.height();
    vi.v_current.write(0);
    vi.ctrl.write(CtrlReg(0));
    vi.origin.write(fb.as_ptr() as u32);
    vi.width.write(0);
    vi.v_intr.write(2);
    vi.burst.write(BurstReg(regs[0]));
    vi.v_sync.write(regs[1]);
    vi.h_sync.write(HSyncReg(regs[2]));
    vi.h_sync_leap.write(HSyncLeapReg(regs[3]));
    vi.h_video.write(HVideoReg(regs[4]));
    vi.v_video.write(VVideoReg(regs[5]));
    vi.v_burst.write(VBurstReg(regs[6]));
    vi.x_scale.write(XScaleReg((0x100 * width as u32) / 160));
    vi.y_scale.write(YScaleReg((0x100 * height as u32) / 60));
    vi.ctrl.write(
        CtrlReg(0)
            .with_depth(ColorDepth::BPP16)
            .with_aa_mode(AntiAliasMode::ResamplingOnly)
            .with_pixel_advance(if system::console_type() != 0 { 2 } else { 3 }),
    );
    wait_vblank(vi);
    vi.width.write(width as u32);
}

#[inline]
fn wait_vblank(vi: &VideoInterface) {
    while (vi.v_current.read() & !1) != 2 {}
    vi.v_current.write(0);
}
