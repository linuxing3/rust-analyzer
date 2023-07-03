#![deny(clippy::pedantic, nonstandard_style)]
#![allow(dead_code)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]

use std::borrow::BorrowMut;
use std::ops::DerefMut;
use std::{env, fs, io};

use imgui::Ui;
use serde_json::json;

use crate::config::Config;

use crate::point3d::Point3D;
use crate::raytracer::ImguiRender;

// Layer trait/interface
pub trait Layer {
    fn on_attach(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    );
    fn on_dettach(
        &mut self,
        ui: &mut Ui,
        size: [f32; 2],
    );
    fn on_update(
        &mut self,
        dt: f32,
    );
    fn on_render(
        &mut self,
        ui: &mut Ui,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    );
}

pub struct RayLayer {
    width: f32,
    height: f32,
    imgui_render: ImguiRender,
    last_rendered_time: f32,
    scene: Config,
}

impl Layer for RayLayer {
    fn on_attach(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    ) {
        let image = self.imgui_render.image_mut();
        image.insert_image_texture(device, queue, renderer);
    }

    fn on_dettach(
        &mut self,
        _ui: &mut Ui,
        _size: [f32; 2],
    ) {
    }

    fn on_update(
        &mut self,
        _dt: f32,
    ) {
        // self.camera.update;
    }

    fn on_render(
        &mut self,
        ui: &mut Ui,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    ) {
        self.imgui_render.resize(&self.scene);
        // prepara image backend data
        self.imgui_render
            .render(device, queue, renderer, &self.scene);
        // render image ui in layer
        self.render_ui(ui);
    }

    // add code here
}

impl RayLayer {
    pub fn new(dt: f32) -> Self {
        let imgui_render = ImguiRender::new();
        let scene = load_scene_from_json().unwrap();
        Self {
            width: 0.0,
            height: 0.0,
            imgui_render,
            scene,
            last_rendered_time: dt,
        }
    }

    pub fn render_ui(
        &mut self,
        ui: &mut imgui::Ui,
    ) {
        unsafe {
            // image window
            let g_title = format!("Gallery");
            let window = ui.window(g_title);
            let mut new_imgui_region_size = None;
            window
                .size([800.0, 600.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    new_imgui_region_size = Some(ui.content_region_avail());
                    ui.text("raytracer");
                    let id = self.imgui_render.texture_id();
                    let pixels = self.imgui_render.image().pixels();
                    if id.id() > 0 && pixels.len() > 0 {
                        imgui::Image::new(id, new_imgui_region_size.unwrap()).build(ui);
                    }
                });

            // controller window
            let c_title = format!("Controller");
            let ctrl_window = ui.window(c_title);
            ctrl_window
                .size([200.0, 200.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    // control samples
                    ui.slider("samples", 1, 128, &mut self.scene.samples_per_pixel);

                    // control camera origin
                    let origin = &mut self.scene.camera.origin;
                    let mut camera_origin_x = origin.x();
                    let mut camera_origin_y = origin.y();
                    let mut camera_origin_z = origin.z();
                    if ui.slider(format!("camera x"), -10.0, 10.0, &mut camera_origin_x) {
                        origin.set_x(camera_origin_x);
                    };
                    if ui.slider(format!("camera y"), -10.0, 10.0, &mut camera_origin_y) {
                        origin.set_y(camera_origin_y);
                    };
                    if ui.slider(format!("camera z"), -10.0, 10.0, &mut camera_origin_z) {
                        origin.set_z(camera_origin_z);
                    };

                    // control sphere center
                    (0..self.scene.objects.len()).for_each(|i| {
                        let id = ui.push_id(i.to_string());

                        let center = &mut self.scene.objects[i].center;
                        // let &mut Point3D { x, y, z } = &mut self.scene.objects[i].center;

                        let mut sphere_x = center.x();
                        let mut sphere_y = center.y();
                        let mut sphere_z = center.z();

                        if ui.slider(format!("Sphere {} x", i), -10.0, 10.0, &mut sphere_x) {
                            center.set_x(sphere_x);
                        }
                        if ui.slider(format!("Sphere {} y", i), -10.0, 10.0, &mut sphere_y) {
                            center.set_y(sphere_y);
                        }
                        if ui.slider(format!("Sphere {} z", i), -10.0, 10.0, &mut sphere_z) {
                            center.set_z(sphere_z);
                        }

                        id.end();
                    });

                    // save current config
                    let mut file_path = String::from("data/current_scene.json");
                    ui.input_text("file path", &mut file_path).build();
                    if ui.button("save config") {
                        save_scene_to_json(&self.scene, &mut file_path).expect("Failt to write");
                    }
                });
        }
    }
}

pub fn load_scene_from_json() -> Result<Config, io::Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} <config_file> <output_file>", args[0]);
    }

    let json = fs::read(&args[1]).expect("Unable to read config file.");
    let scene = serde_json::from_slice::<Config>(&json).expect("Unable to parse config json");

    Ok(scene)
}

pub fn save_scene_to_json(
    config: &Config,
    path: &mut str,
) -> Result<(), io::Error> {
    let scene_json = serde_json::to_string(config)?;
    fs::write(path, scene_json).expect("Unable to read config file.");

    Ok(())
}
