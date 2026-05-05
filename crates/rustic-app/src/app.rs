//! Top-level winit application. See `PLAN.md` Sections 7 and 11.
//!
//! Uses winit 0.30's `ApplicationHandler`: window + surface are created
//! lazily on `resumed` so the same code path works on Android (which
//! also pauses/resumes the surface). Surface is `'static` because the
//! window is held in an `Arc`.

use crate::boot::{init_logging, install_panic_hook};
use crate::input_bridge::{build_event, map_key};
use crate::screen::ScreenStack;
use anyhow::Result;
use rustic_audio::Mixer;
use rustic_render::{
    CameraRegistry, Composite, RenderCommandList, RenderState, SpriteBatcher, SpritePipeline,
    SurfaceConfig, Texture,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AppOptions {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            title: "RusticV3",
            width: 1280,
            height: 720,
        }
    }
}

/// Runtime state held inside the event loop.
struct App {
    options: AppOptions,
    boot_instant: Instant,
    mixer: Mixer,
    cameras: CameraRegistry,
    cmds: RenderCommandList,
    atlases: HashMap<rustic_core::ids::AssetId, Texture>,
    batcher: SpriteBatcher,
    screens: ScreenStack,
    runtime: Option<Runtime>,
}

struct Runtime {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_cfg: SurfaceConfig,
    rs: RenderState,
    pipeline: SpritePipeline,
    composite: Composite,
}

impl App {
    fn new(options: AppOptions) -> Self {
        Self {
            options,
            boot_instant: Instant::now(),
            mixer: Mixer::new(48_000),
            cameras: CameraRegistry::with_default_fnf(),
            cmds: RenderCommandList::new(),
            atlases: HashMap::new(),
            batcher: SpriteBatcher::new(),
            screens: ScreenStack::new(),
            runtime: None,
        }
    }

    fn create_runtime(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let attrs = WindowAttributes::default()
            .with_title(self.options.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.options.width,
                self.options.height,
            ));
        let window = Arc::new(event_loop.create_window(attrs)?);

        // wgpu Instance must be built before we make the surface so we
        // can pass `compatible_surface` to the adapter request.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| anyhow::anyhow!("create_surface: {e}"))?;

        let rs = pollster::block_on(RenderState::new_async(Some(&surface)))?;
        let inner = window.inner_size();
        let surface_cfg = rs.configure_surface(
            &surface,
            inner.width,
            inner.height,
            wgpu::PresentMode::AutoVsync,
        )?;
        let pipeline = SpritePipeline::new(&rs.device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let composite = Composite::new(&rs, surface_cfg.format);

        // Discard the standalone instance — the adapter owns its own
        // lineage through `RenderState`.
        drop(instance);

        self.runtime = Some(Runtime {
            window,
            surface,
            surface_cfg,
            rs,
            pipeline,
            composite,
        });
        Ok(())
    }

    fn redraw(&mut self) {
        let Some(rt) = self.runtime.as_mut() else {
            return;
        };
        let frame = match rt.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                rt.surface.configure(
                    &rt.rs.device,
                    &wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format: rt.surface_cfg.format,
                        width: rt.surface_cfg.width.max(1),
                        height: rt.surface_cfg.height.max(1),
                        present_mode: rt.surface_cfg.present_mode,
                        alpha_mode: wgpu::CompositeAlphaMode::Auto,
                        view_formats: vec![],
                        desired_maximum_frame_latency: 2,
                    },
                );
                return;
            }
            Err(e) => {
                tracing::warn!(target: "rustic.render", "surface error: {e:?}");
                return;
            }
        };

        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = rt
            .rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("rustic.frame.encoder"),
            });
        let bg = wgpu::Color {
            r: 0.07,
            g: 0.07,
            b: 0.10,
            a: 1.0,
        };

        // 1. Sprite pass into the 1280x720 reference target.
        self.batcher.draw_to_reference(
            &rt.rs,
            &mut encoder,
            &rt.pipeline,
            &self.cameras,
            &self.atlases,
            self.cmds.as_slice(),
            bg,
        );
        // 2. Composite reference -> swapchain with letterbox.
        rt.composite.encode(
            &mut encoder,
            &target,
            rt.surface_cfg.width,
            rt.surface_cfg.height,
            wgpu::Color::BLACK,
        );
        rt.rs.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn handle_resize(&mut self, w: u32, h: u32) {
        let Some(rt) = self.runtime.as_mut() else {
            return;
        };
        rt.surface_cfg.width = w;
        rt.surface_cfg.height = h;
        rt.surface.configure(
            &rt.rs.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: rt.surface_cfg.format,
                width: w.max(1),
                height: h.max(1),
                present_mode: rt.surface_cfg.present_mode,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.is_some() {
            return;
        }
        if let Err(e) = self.create_runtime(event_loop) {
            tracing::error!(target: "rustic", "failed to bring up renderer: {e:#}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.handle_resize(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(action) = map_key(event.physical_key) {
                    let evt = build_event(action, event.state, self.boot_instant, &self.mixer);
                    self.screens.input(&evt);
                    if event.state == ElementState::Pressed
                        && action == rustic_core::InputAction::Back
                    {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if let Some(rt) = self.runtime.as_ref() {
                    rt.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Public entry point. Initializes logging + panic hook, builds the app
/// state, runs the winit event loop. Returns when the user closes the
/// window or the loop exits.
pub fn run(options: AppOptions) -> Result<()> {
    init_logging();
    install_panic_hook();
    tracing::info!(target: "rustic", "starting RusticV3 ({}x{})", options.width, options.height);

    let event_loop = EventLoop::new()?;
    let mut app = App::new(options);
    event_loop.run_app(&mut app)?;
    Ok(())
}
