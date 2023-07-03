#![allow(dead_code)]
#![allow(unused_imports)]
use imgui::TextureId;

use thiserror::Error;

use image::{GenericImage, ImageBuffer, Rgb, Rgba, RgbaImage};

pub type XImageBuffer = ImageBuffer<Rgb<u8>, Vec<u8>>;
pub type YImageBuffer = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub struct Texture {
    dimensions: (u32, u32),
    data: Vec<[f32; 3]>,
}

pub struct WgpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn as_slice(&self) -> &[[f32; 3]] {
        self.data.as_slice()
    }

    pub fn as_vec_u8(&self) -> Vec<u8> {
        self.data
            .as_slice()
            .into_iter()
            .map(|s| -> [u8; 3] {
                [
                    (s[0] * 255_f32) as u8,
                    (s[1] * 255_f32) as u8,
                    (s[2] * 255_f32) as u8,
                ]
            })
            .flatten()
            .collect()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }
}

impl WgpuTexture {
    pub fn new_imgui_texture<'a>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderer: &imgui_wgpu::Renderer,
        raw_data: &[u8],
        size: wgpu::Extent3d,
    ) -> imgui_wgpu::Texture {
        let texture_config = imgui_wgpu::TextureConfig {
            size,
            label: Some("raw texture"),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        };

        let texture = imgui_wgpu::Texture::new(device, renderer, texture_config);

        texture.write(&queue, &raw_data, size.width, size.height);

        texture
    }

    pub fn new_wgpu_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: Option<&str>,
    ) -> Result<Self, TextureError> {
        let img = image::load_from_memory(bytes)?;

        let dimensions = img.dimensions();

        let rgba = img.to_rgba();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}

#[derive(Error, Debug)]

pub enum TextureError {
    #[error(transparent)]
    FileIoError(#[from] std::io::Error),
    #[error(transparent)]
    ImageLoadError(#[from] image::ImageError),
}
