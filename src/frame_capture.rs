//! Optional framebuffer capture for presentation QA, including the wallpaper HUD.
use std::{io::Write, path::Path};

pub fn encode(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
) -> (wgpu::Buffer, u32) {
    let pitch = (texture.width() * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("presentation capture"),
        size: u64::from(pitch) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    (buffer, pitch)
}

pub fn save(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    pitch: u32,
    config: &wgpu::SurfaceConfiguration,
    path: &Path,
) -> Result<(), String> {
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let data = slice.get_mapped_range();
    let bgra = matches!(
        config.format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    let mut image = format!("P6\n{} {}\n255\n", config.width, config.height).into_bytes();
    for row in data.chunks(pitch as usize) {
        for pixel in row[..config.width as usize * 4].chunks_exact(4) {
            if bgra {
                image.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
            } else {
                image.extend_from_slice(&pixel[..3]);
            }
        }
    }
    drop(data);
    buffer.unmap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&image).map_err(|e| e.to_string())
}
