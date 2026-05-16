use super::App;
use crate::app_runtime::reconfigure_surface;

impl App {
    pub(super) fn redraw(&mut self) {
        self.update_debug_frame_time();
        self.rebuild_frame_commands();
        let Some(rt) = self.runtime.as_mut() else {
            return;
        };
        let frame = match rt.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let (width, height) = (rt.surface_cfg.width, rt.surface_cfg.height);
                reconfigure_surface(rt, width, height);
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

        if let Some(text) = rt.text.as_mut() {
            if let Err(e) = text.prepare(&rt.rs, self.text_cmds.as_slice()) {
                tracing::warn!(target: "rustic.render", "text prepare failed: {e}");
            }
        }
        if let Err(e) = self.batcher.draw_to_reference(
            &rt.rs,
            &mut encoder,
            &rt.pipeline,
            &self.cameras,
            &self.atlases,
            self.cmds.as_slice(),
            wgpu::Color::BLACK,
            rt.text.as_ref(),
        ) {
            tracing::warn!(target: "rustic.render", "sprite/text pass failed: {e}");
        }
        if let Some(text) = rt.text.as_mut() {
            text.trim_atlas();
        }
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

    pub(super) fn handle_resize(&mut self, w: u32, h: u32) {
        let Some(rt) = self.runtime.as_mut() else {
            return;
        };
        reconfigure_surface(rt, w, h);
    }
}
