//! The window: draws the latest frame, with its border, scaled by the GPU.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use super::Shared;
use starquake::controls::Input;
use starquake::display::{BITMAP_LEN, HEIGHT, WIDTH};

const BORDER: usize = 32;
pub const FULL_W: usize = WIDTH + 2 * BORDER;
pub const FULL_H: usize = HEIGHT + 2 * BORDER;
const SCALE: f64 = 3.0;

const PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xD8],
    [0xD8, 0x00, 0x00],
    [0xD8, 0x00, 0xD8],
    [0x00, 0xD8, 0x00],
    [0x00, 0xD8, 0xD8],
    [0xD8, 0xD8, 0x00],
    [0xD8, 0xD8, 0xD8],
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xFF],
    [0xFF, 0x00, 0x00],
    [0xFF, 0x00, 0xFF],
    [0x00, 0xFF, 0x00],
    [0x00, 0xFF, 0xFF],
    [0xFF, 0xFF, 0x00],
    [0xFF, 0xFF, 0xFF],
];

/// Draws Spectrum display memory with a border into an RGBA frame.
pub fn draw(mem: &[u8], border: u8, frame: u64, out: &mut [u8]) {
    let b = PALETTE[(border & 7) as usize];
    for px in out.as_chunks_mut::<4>().0 {
        px.copy_from_slice(&[b[0], b[1], b[2], 0xFF]);
    }
    let flash = (frame / 16) % 2 == 1;
    for y in 0..HEIGHT {
        let line = ((y & 0xC0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2);
        for col in 0..32 {
            let bits = mem[line + col];
            let attr = mem[BITMAP_LEN + (y / 8) * 32 + col];
            let bright = ((attr >> 6) & 1) as usize * 8;
            let (mut ink, mut paper) = ((attr & 7) as usize + bright, ((attr >> 3) & 7) as usize + bright);
            if attr & 0x80 != 0 && flash {
                std::mem::swap(&mut ink, &mut paper);
            }
            for bit in 0..8 {
                let c = PALETTE[if bits & (0x80 >> bit) != 0 { ink } else { paper }];
                let i = ((y + BORDER) * FULL_W + BORDER + col * 8 + bit) * 4;
                out[i..i + 3].copy_from_slice(&c);
            }
        }
    }
}

struct App {
    shared: Arc<Shared>,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    error: Option<String>,
    /// The host keys physically down, which the machine's input is built
    /// from. winit only synthesises key-ups on focus loss on some platforms,
    /// so this is cleared when the window stops listening.
    held: HashSet<KeyCode>,
    /// The game frame last painted, so the same one is not painted twice.
    shown: u64,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Starquake")
            .with_inner_size(LogicalSize::new(FULL_W as f64 * SCALE, FULL_H as f64 * SCALE))
            .with_min_inner_size(LogicalSize::new(FULL_W as f64, FULL_H as f64));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        // Some compositors report 0x0 before the first configure, and wgpu
        // panics when a surface is configured that size.
        let (w, h) = (size.width.max(1), size.height.max(1));
        let surface = SurfaceTexture::new(w, h, window.clone());
        match Pixels::new(FULL_W as u32, FULL_H as u32, surface) {
            Ok(p) => self.pixels = Some(p),
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.shared.quit.store(true, Ordering::Relaxed);
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key
                    && !event.repeat
                {
                    if event.state == ElementState::Pressed {
                        self.held.insert(code);
                    } else {
                        self.held.remove(&code);
                    }
                    *self.shared.input.lock().unwrap() = super::input::build(&self.held);
                }
            }
            // Nothing is held once the window is not listening. Without this,
            // a key held while switching away stays down for ever on the
            // platforms where winit sends no key-ups.
            WindowEvent::Focused(false) => {
                self.held.clear();
                *self.shared.input.lock().unwrap() = Input::default();
            }
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(p) = &mut self.pixels {
                    {
                        let screen = self.shared.screen.lock().unwrap();
                        draw(&screen.0, screen.1, screen.2, p.frame_mut());
                    }
                    if let Err(e) = p.render() {
                        self.error = Some(e.to_string());
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // The game side sets this when it finishes, and when it stops
        // unexpectedly; either way the window should not outlive it.
        if self.shared.quit.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        // Paint only when the game has produced a new frame, and let the loop
        // sleep in between. Asking for a redraw every time round instead ties
        // the rate to how long `render` blocks, and the moment the window is
        // occluded it stops blocking at all: that spun this thread at over
        // 20,000 repaints a second, burning a core the game needs.
        let latest = self.shared.screen.lock().unwrap().2;
        if latest != self.shown {
            self.shown = latest;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(2),
        ));
    }
}

pub fn run(shared: Arc<Shared>) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        shared,
        window: None,
        pixels: None,
        error: None,
        shown: u64::MAX,
        held: HashSet::new(),
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    if let Some(e) = app.error {
        return Err(e);
    }
    if app.shared.dead.load(Ordering::Relaxed) {
        return Err("the game stopped unexpectedly; see the panic above".into());
    }
    Ok(())
}
