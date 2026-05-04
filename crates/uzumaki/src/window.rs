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
    current_cursor: UzCursorIcon,
    valid_surface: bool,
    vello_target: Option<(wgpu::Texture, wgpu::TextureView)>,
}

impl Window {
    pub fn new(gpu: &GpuContext, winit_window: Arc<WinitWindow>) -> Result<Self> {
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

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![format],
        };

        surface.configure(&gpu.device, &surface_config);

        // let renderer = vello::Renderer::new(&gpu.device, RendererOptions::default())
        //     .context("Error creating renderer")?;
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
            current_cursor: UzCursorIcon::Default,
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

    pub(crate) fn paint_and_present(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dom: &mut UIState,
    ) {
        if !self.valid_surface {
            return;
        }

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

        // Prepaint walks the freshly rebuilt layout tree, refreshes hit/scroll
        // caches and yields a pure paint list. Hover changes take effect on the
        // next redraw instead of forcing a second prepaint in this frame.
        let list = Painter::new(dom, &mut self.text_renderer, scale).prepaint();
        dom.refresh_hit_test();
        list.paint(&mut self.scene, &mut self.text_renderer);

        let target_view = Self::ensure_vello_target(&mut self.vello_target, device, width, height);

        let render_params = RenderParams {
            base_color: Color::from_rgba8(24, 24, 37, 255),
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

    fn resize_surface(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(device, &self.surface_config);
        self.vello_target = None;
    }

    pub(crate) fn on_resize(&mut self, device: &wgpu::Device, width: u32, height: u32) -> bool {
        if width != 0 && height != 0 {
            self.resize_surface(device, width, height);
            self.valid_surface = true;
            true
        } else {
            self.valid_surface = false;
            false
        }
    }
}
