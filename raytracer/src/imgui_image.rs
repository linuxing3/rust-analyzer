#![allow(dead_code)]
#![allow(unused)]
#![allow(unused_imports)]

use imgui::TextureId;

#[derive(Debug)]
pub struct ImguiImage {
    texture_id: TextureId,
    pixels: Box<Vec<u8>>,
    image_width: usize,
    image_height: usize,
}

impl ImguiImage {
    pub fn new() -> Self {
        let texture_id = TextureId::new(0);
        let image_width = 1;
        let image_height = 1;

        let mut data = Vec::new();
        data.resize(4 * image_width * image_height, 0);
        let pixels = Box::new(data);
        Self {
            texture_id,
            pixels, 
            image_width,
            image_height,
        }
    }

    pub fn resize(
        &mut self,
        width: usize,
        height: usize,
    ) {
        if self.image_width != width || self.image_height != height {
            self.image_width = width;
            self.image_height = height;
        }
    }

    pub fn update_image_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    ) {
        // update texture id
        let imgui_texture = self.generate_texture(device, queue, renderer);
        renderer.textures.replace(self.texture_id(), imgui_texture);
    }

    pub fn insert_image_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    ) {
        // insert new image texture
        let imgui_texture = self.generate_texture(device, queue, renderer);
        self.texture_id = renderer.textures.insert(imgui_texture);
    }

    pub fn generate_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &mut imgui_wgpu::Renderer,
    ) -> imgui_wgpu::Texture {
        let width = self.image_width as u32;
        let height = self.image_height as u32;
        let texture_config = imgui_wgpu::TextureConfig {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            label: Some("raw texture"),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        };

        let texture = imgui_wgpu::Texture::new(device, renderer, texture_config);

        texture.write(&queue, &self.pixels(), width, height);
        texture
    }

    pub fn texture_id(&self) -> TextureId {
        self.texture_id
    }

    pub fn set_pixels(
        &mut self,
        pixels: Box<Vec<u8>>,
    ) {
        self.pixels = pixels;
    }

    pub fn pixels(&self) -> &Vec<u8> {
        self.pixels.as_ref()
    }

    pub fn pixels_mut(&mut self) -> &mut Box<Vec<u8>> {
        &mut self.pixels
    }
    
}
