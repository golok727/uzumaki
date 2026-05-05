use anyhow::{Context, Result};
use std::sync::Arc;
use vello::peniko::Color;
use vello::{AaSupport, RenderParams, RendererOptions, Scene};

use winit::window::Window as WinitWindow;

use crate::cursor::UzCursorIcon;
use crate::element::render::Painter;
use crate::gpu::GpuContext;
use crate::text::TextRenderer;
use crate::ui::UIState;

pub struct Window {
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) winit_window: Arc<WinitWindow>,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
    pub(crate) renderer: vello::Renderer,
    pub(crate) scene: Scene,
    pub(crate) text_renderer: TextRenderer,
    gpu: GpuContext,
    current_cursor: UzCursorIcon,
    transparent: bool,
    valid_surface: bool,
    vello_target: Option<(wgpu::Texture, wgpu::TextureView)>,
}

impl Window {
    pub fn new(
        gpu: &GpuContext,
        winit_window: Arc<WinitWindow>,
        transparent: bool,
    ) -> Result<Self> {
        let surface = gpu
            .instance
            .create_surface(winit_window.clone())
            .context("Error creating surface")?;

        let size = winit_window.inner_size();

        let valid_surface = size.width != 0 && size.height != 0;

        let surface_caps = surface.get_capabilities(&gpu.adapter);
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
        let alpha_mode = choose_alpha_mode(&surface_caps.alpha_modes, transparent);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![format],
        };

        surface.configure(&gpu.device, &surface_config);

        let renderer = vello::Renderer::new(
            &gpu.device,
            RendererOptions {
                antialiasing_support: AaSupport::area_only(),
                ..Default::default()
            },
        )
        .context("Error creating renderer")?;

        let scene = Scene::new();

        Ok(Self {
            winit_window,
            renderer,
            surface,
            surface_config,
            scene,
            text_renderer: TextRenderer::new(),
            gpu: gpu.clone(),
            current_cursor: UzCursorIcon::Default,
            transparent,
            valid_surface,
            vello_target: None,
        })
    }

    pub fn id(&self) -> winit::window::WindowId {
        self.winit_window.id()
    }

    pub fn scale_factor(&self) -> f64 {
        self.winit_window.scale_factor()
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        if self.transparent == transparent {
            return;
        }
        self.transparent = transparent;
        self.reconfigure_surface_alpha();
        if self.valid_surface {
            self.surface
                .configure(&self.gpu.device, &self.surface_config);
        }
        self.winit_window.set_transparent(transparent);
        self.winit_window.request_redraw();
    }

    pub fn inner_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.winit_window.inner_size()
    }

    pub(crate) fn set_cursor(&mut self, icon: UzCursorIcon) {
        if self.current_cursor == icon {
            return;
        }
        self.current_cursor = icon;
        self.winit_window.set_cursor(icon.to_winit());
    }

    pub(crate) fn paint_and_present(&mut self, dom: &mut UIState) {
        if !self.valid_surface {
            return;
        }

        let device = &self.gpu.device;
        let queue = &self.gpu.queue;
        let width = self.surface_config.width;
        let height = self.surface_config.height;

        self.scene.reset();

        let scale = self.winit_window.scale_factor();
        // Layout uses logical pixels; rendering uses physical via Affine::scale
        dom.compute_layout(
            width as f32 / scale as f32,
            height as f32 / scale as f32,
            &mut self.text_renderer,
        );

        Painter::new(dom, &mut self.text_renderer, scale).paint(&mut self.scene);
        dom.refresh_hit_test();

        let target_view = Self::ensure_vello_target(&mut self.vello_target, device, width, height);

        let render_params = RenderParams {
            base_color: if self.transparent {
                Color::from_rgba8(0, 0, 0, 0)
            } else {
                Color::from_rgba8(24, 24, 37, 255)
            },
            width,
            height,
            antialiasing_method: vello::AaConfig::Area,
        };
        self.renderer
            .render_to_texture(device, queue, &self.scene, target_view, &render_params)
            .expect("Failed to render");

        // Blit to surface
        let surface_texture = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(device, &self.surface_config);
                match self.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Failed to get surface texture: {e}");
                        return;
                    }
                }
            }
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_config.format),
                ..Default::default()
            });

        let blitter = wgpu::util::TextureBlitter::new(device, self.surface_config.format);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        blitter.copy(device, &mut encoder, target_view, &surface_view);
        queue.submit([encoder.finish()]);
        surface_texture.present();
    }

    fn ensure_vello_target<'a>(
        target: &'a mut Option<(wgpu::Texture, wgpu::TextureView)>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> &'a wgpu::TextureView {
        if target.is_none() {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vello_target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            *target = Some((texture, view));
        }
        &target.as_ref().unwrap().1
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.reconfigure_surface_alpha();
        self.surface
            .configure(&self.gpu.device, &self.surface_config);
        self.vello_target = None;
    }

    pub(crate) fn on_resize(&mut self, width: u32, height: u32) -> bool {
        if width != 0 && height != 0 {
            self.resize_surface(width, height);
            self.valid_surface = true;
            true
        } else {
            self.valid_surface = false;
            false
        }
    }

    fn reconfigure_surface_alpha(&mut self) {
        let surface_caps = self.surface.get_capabilities(&self.gpu.adapter);
        self.surface_config.alpha_mode =
            choose_alpha_mode(&surface_caps.alpha_modes, self.transparent);
    }
}

fn choose_alpha_mode(
    modes: &[wgpu::CompositeAlphaMode],
    transparent: bool,
) -> wgpu::CompositeAlphaMode {
    use wgpu::CompositeAlphaMode::*;

    let preferred: &[wgpu::CompositeAlphaMode] = if transparent {
        &[PreMultiplied, PostMultiplied, Inherit]
    } else {
        &[Opaque]
    };

    preferred
        .iter()
        .find(|mode| modes.contains(mode))
        .or_else(|| modes.first())
        .copied()
        .unwrap_or(Auto)
}

#[cfg(test)]
mod tests {
    use super::choose_alpha_mode;

    #[test]
    fn transparent_windows_prefer_compositing_alpha_modes() {
        let modes = [
            wgpu::CompositeAlphaMode::Opaque,
            wgpu::CompositeAlphaMode::PostMultiplied,
        ];

        assert_eq!(
            choose_alpha_mode(&modes, true),
            wgpu::CompositeAlphaMode::PostMultiplied
        );
    }

    #[test]
    fn opaque_windows_prefer_opaque_alpha_mode() {
        let modes = [
            wgpu::CompositeAlphaMode::Inherit,
            wgpu::CompositeAlphaMode::Opaque,
        ];

        assert_eq!(
            choose_alpha_mode(&modes, false),
            wgpu::CompositeAlphaMode::Opaque
        );
    }

    #[test]
    fn alpha_mode_selection_falls_back_to_supported_mode() {
        let modes = [wgpu::CompositeAlphaMode::Opaque];

        assert_eq!(
            choose_alpha_mode(&modes, true),
            wgpu::CompositeAlphaMode::Opaque
        );
    }
}
