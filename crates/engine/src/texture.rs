//! Decodificação de texturas PNG em Safe Rust e integração com memória de GPU.
#![forbid(unsafe_code)]

use miniquad::graphics::{RenderingBackend, TextureId};
use std::fs;
use std::path::Path;

/// Imagem bruta em memória RAM decodificada no formato RGBA8 (4 bytes por pixel).
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, String> {
        let expected = (width as usize) * (height as usize) * 4;
        if pixels.len() != expected {
            return Err(format!(
                "Tamanho inválido de pixels: esperado {} bytes ({}x{}x4), recebido {} bytes",
                expected,
                width,
                height,
                pixels.len()
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Cria uma imagem preenchida com uma cor RGBA uniforme.
    pub fn solid(width: u32, height: u32, color: [u8; 4]) -> Self {
        let count = (width as usize) * (height as usize);
        let mut pixels = Vec::with_capacity(count * 4);
        for _ in 0..count {
            pixels.extend_from_slice(&color);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Gera uma textura quadriculada (*checkerboard*) de fallback para quando o arquivo PNG não for encontrado.
    pub fn checkerboard(width: u32, height: u32, cell_size: u32, c1: [u8; 4], c2: [u8; 4]) -> Self {
        let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 4);
        let cell = cell_size.max(1);

        for y in 0..height {
            for x in 0..width {
                let is_c1 = ((x / cell) + (y / cell)) % 2 == 0;
                let color = if is_c1 { c1 } else { c2 };
                pixels.extend_from_slice(&color);
            }
        }

        Self {
            width,
            height,
            pixels,
        }
    }

    /// Decodifica uma imagem PNG a partir de um buffer de bytes em Safe Rust.
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, String> {
        let decoder = png::Decoder::new(bytes);
        let mut reader = decoder.read_info().map_err(|e| format!("Erro no cabeçalho PNG: {}", e))?;

        let mut raw_buf = vec![0; reader.output_buffer_size()];
        let output_info = reader
            .next_frame(&mut raw_buf)
            .map_err(|e| format!("Erro ao decodificar frame PNG: {}", e))?;

        let width = output_info.width;
        let height = output_info.height;
        let data = &raw_buf[..output_info.buffer_size()];

        let pixels = match output_info.color_type {
            png::ColorType::Rgba => data.to_vec(),
            png::ColorType::Rgb => {
                let pixel_count = (width as usize) * (height as usize);
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for chunk in data.chunks_exact(3) {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                rgba
            }
            png::ColorType::Grayscale => {
                let pixel_count = (width as usize) * (height as usize);
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for &gray in data {
                    rgba.extend_from_slice(&[gray, gray, gray, 255]);
                }
                rgba
            }
            png::ColorType::GrayscaleAlpha => {
                let pixel_count = (width as usize) * (height as usize);
                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for chunk in data.chunks_exact(2) {
                    rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
                }
                rgba
            }
            other => return Err(format!("Formato de cor PNG não suportado: {:?}", other)),
        };

        Self::new(width, height, pixels)
    }

    /// Lê e decodifica arquivo PNG do disco se existir, ou gera fallback procedural se não existir.
    pub fn load_or_fallback(
        path: impl AsRef<Path>,
        fallback_w: u32,
        fallback_h: u32,
    ) -> (Self, bool) {
        let p = path.as_ref();
        if p.exists() {
            if let Ok(bytes) = fs::read(p) {
                if let Ok(img) = Self::from_png_bytes(&bytes) {
                    return (img, true);
                }
            }
        }

        // Fallback procedural visualmente distinguível (magenta/preto clássico de asset ausente)
        let fallback = Self::checkerboard(
            fallback_w,
            fallback_h,
            16,
            [255, 0, 255, 255], // Magenta
            [20, 20, 20, 255],  // Cinza escuro
        );
        (fallback, false)
    }

    /// Envia a imagem para a GPU e retorna o manipulador `TextureId` do Miniquad.
    pub fn upload_to_gpu(&self, backend: &mut Box<dyn RenderingBackend>) -> TextureId {
        backend.new_texture_from_rgba8(self.width as u16, self.height as u16, &self.pixels)
    }

    /// Obtém cor do pixel em coordenadas (x, y).
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        Some([
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_image_checkerboard() {
        let img = RgbaImage::checkerboard(64, 64, 8, [255, 255, 255, 255], [0, 0, 0, 255]);
        assert_eq!(img.width, 64);
        assert_eq!(img.height, 64);
        assert_eq!(img.pixels.len(), 64 * 64 * 4);

        // Pixel no bloco (0, 0) deve ser branco
        assert_eq!(img.get_pixel(2, 2), Some([255, 255, 255, 255]));
        // Pixel no bloco (1, 0) deve ser preto
        assert_eq!(img.get_pixel(10, 2), Some([0, 0, 0, 255]));
    }

    #[test]
    fn test_load_or_fallback_missing_file() {
        let (img, loaded_from_disk) =
            RgbaImage::load_or_fallback("caminho_inexistente_assets.png", 128, 128);
        assert!(!loaded_from_disk);
        assert_eq!(img.width, 128);
        assert_eq!(img.height, 128);
    }
}
