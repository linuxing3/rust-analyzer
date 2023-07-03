#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::{collections::VecDeque, time::Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    layer::{Layer, RayLayer},
};

#[derive(Debug)]
pub struct App {
    title: String,
}

impl App {
    pub fn new(title: String) -> Self {
        Self { title }
    }
    pub fn run() {
        run();
    }
}

pub fn run() {
    let event_loop = EventLoop::new();
    
    let window = WindowBuilder::new()
        .with_title("weekend-raytracer-wgpu")
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    let mut context = pollster::block_on(GpuContext::new(&window));

    let _viewport_size = {
        let viewport = window.inner_size();

        (viewport.width, viewport.height)
    };

    let _max_viewport_resolution = window
        .available_monitors()
        .map(|monitor| -> u32 {
            let size = monitor.size();

            size.width * size.height
        })
        .max()
        .expect("There should be at least one monitor available");

    let mut imgui = imgui::Context::create();

    let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);

    imgui_platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Rounded,
    );

    imgui.set_ini_filename(Some(std::path::PathBuf::from("imgui.ini")));

    let hidpi_factor = window.scale_factor() as f32;

    let font_size = 13.0 * hidpi_factor;

    imgui.io_mut().font_global_scale = 1.0 / hidpi_factor;

    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

    let imgui_renderer_config = imgui_wgpu::RendererConfig {
        texture_format: context.surface_config.format,
        ..Default::default()
    };

    let mut imgui_renderer = imgui_wgpu::Renderer::new(
        &mut imgui,
        &context.device,
        &context.queue,
        imgui_renderer_config,
    );

    let mut last_cursor = None;

    let mut last_time = Instant::now();

    let mut fps_counter = FpsCounter::new();

    // IMGUI
    let mut layer = RayLayer::new(0.0);
    layer.on_attach(&context.device, &context.queue, &mut imgui_renderer);

    event_loop.run(move |event, _, _control_flow| {
        imgui_platform.handle_event(imgui.io_mut(), &window, &event);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *_control_flow = ControlFlow::Exit;
                }

                WindowEvent::Resized(physical_size) => {
                    if physical_size.width > 0 && physical_size.height > 0 {
                        context.surface_config.width = physical_size.width;

                        context.surface_config.height = physical_size.height;

                        context
                            .surface
                            .configure(&context.device, &context.surface_config);
                    }
                }

                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    if new_inner_size.width > 0 && new_inner_size.height > 0 {
                        context.surface_config.width = new_inner_size.width;

                        context.surface_config.height = new_inner_size.height;

                        context
                            .surface
                            .configure(&context.device, &context.surface_config);
                    }
                }

                _ => {}
            },

            Event::NewEvents(..) => {
                let dt = last_time.elapsed().as_secs_f32();
                let now = Instant::now();

                fps_counter.update(dt);

                imgui.io_mut().update_delta_time(now - last_time);

                last_time = now;
            }
            Event::MainEventsCleared => {
                {
                    imgui_platform
                        .prepare_frame(imgui.io_mut(), &window)
                        .expect("WinitPlatform::prepare_frame failed");

                    let mut ui = imgui.frame();

                    layer.on_render(
                        &mut ui,
                        &context.device,
                        &context.queue,
                        &mut imgui_renderer,
                    );

                    if last_cursor != Some(ui.mouse_cursor()) {
                        last_cursor = Some(ui.mouse_cursor());

                        imgui_platform.prepare_render(&ui, &window);
                    }
                }

                window.request_redraw();
            }

            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let frame = match context.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("Surface error: {:?}", e);

                        return;
                    }
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.012,
                                    g: 0.012,
                                    b: 0.012,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                        label: None,
                    });

                    match imgui_renderer.render(
                        imgui.render(),
                        &context.queue,
                        &context.device,
                        &mut render_pass,
                    ) {
                        Err(e) => eprintln!("Imgui render error: {:?}", e),
                        _ => {}
                    }
                }

                context.queue.submit(Some(encoder.finish()));

                frame.present();
            }

            _ => {}
        }
    });
}

pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    async fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe {
            instance
                .create_surface(window)
                .expect("Surface creation should succeed on desktop")
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Adapter should be available on desktop");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits {
                        max_storage_buffer_binding_size: 512_u32 << 20,
                        ..Default::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .expect("Device should be available on desktop");

        let window_size = window.inner_size();

        // NOTE
        // You can check supported configs, such as formats, by calling
        // Surface::capabilities
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
        };

        surface.configure(&device, &surface_config);

        Self {
            device,
            queue,
            surface,
            surface_config,
        }
    }
}

struct FpsCounter {
    frame_times: VecDeque<f32>,
}

impl FpsCounter {
    const MAX_FRAME_TIMES: usize = 8;

    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(Self::MAX_FRAME_TIMES),
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
    ) {
        self.frame_times.push_back(dt);

        if self.frame_times.len() > Self::MAX_FRAME_TIMES {
            self.frame_times.pop_front();
        }
    }

    pub fn average_fps(&self) -> f32 {
        let sum: f32 = self.frame_times.iter().sum();

        self.frame_times.len() as f32 / sum
    }
}
