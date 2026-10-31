//! Renderizador de texto leve e fontes bitmap em Safe Rust.
#![forbid(unsafe_code)]

use crate::batch::SpriteBatch;
use crate::geometry::Rect;
use crate::graphics::Color;
use crate::texture::RgbaImage;
use miniquad::graphics::{RenderingBackend, TextureId};
use std::collections::HashMap;

/// Metadados de um glifo individual dentro da folha de textura da fonte.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    pub character: char,
    pub uv_min: (f32, f32),
    pub uv_max: (f32, f32),
    pub width: f32,
    pub height: f32,
    pub advance: f32,
}

/// Fonte bitmap com atlas de glifos embutido e layout tipográfico.
#[derive(Debug, Clone)]
pub struct BitmapFont {
    pub texture_image: RgbaImage,
    pub texture_id: Option<TextureId>,
    pub glyphs: HashMap<char, Glyph>,
    pub base_width: f32,
    pub base_height: f32,
    pub line_spacing: f32,
}

impl BitmapFont {
    /// Constrói a fonte padrão embutida com caracteres ASCII nítidos (5x7 em célula 8x8).
    pub fn default_font() -> Self {
        const TEX_W: u32 = 128;
        const TEX_H: u32 = 64;
        const CELL_SIZE: u32 = 8;
        const COLS: u32 = TEX_W / CELL_SIZE; // 16 colunas

        let mut image = RgbaImage::solid(TEX_W, TEX_H, [0, 0, 0, 0]);
        let mut glyphs = HashMap::new();

        // Caracteres suportados pelo atlas padrão (ASCII 32 a 126)
        for char_code in 32u8..=126u8 {
            let c = char_code as char;
            let idx = (char_code - 32) as u32;
            let col = idx % COLS;
            let row = idx / COLS;

            let cell_x = col * CELL_SIZE;
            let cell_y = row * CELL_SIZE;

            // Rasteriza os bits 5x7 na célula
            let bitmap = get_glyph_bitmap_5x7(c);
            for (col_idx, &col_bits) in bitmap.iter().enumerate() {
                for bit_row in 0..7 {
                    if (col_bits >> bit_row) & 1 == 1 {
                        let px = cell_x + col_idx as u32;
                        let py = cell_y + bit_row;
                        let offset = ((py * TEX_W + px) * 4) as usize;
                        image.pixels[offset] = 255;
                        image.pixels[offset + 1] = 255;
                        image.pixels[offset + 2] = 255;
                        image.pixels[offset + 3] = 255;
                    }
                }
            }

            let u_min = cell_x as f32 / TEX_W as f32;
            let v_min = cell_y as f32 / TEX_H as f32;
            let u_max = (cell_x + 6) as f32 / TEX_W as f32;
            let v_max = (cell_y + 8) as f32 / TEX_H as f32;

            glyphs.insert(
                c,
                Glyph {
                    character: c,
                    uv_min: (u_min, v_min),
                    uv_max: (u_max, v_max),
                    width: 6.0,
                    height: 8.0,
                    advance: 6.0,
                },
            );
        }

        Self {
            texture_image: image,
            texture_id: None,
            glyphs,
            base_width: 6.0,
            base_height: 8.0,
            line_spacing: 10.0,
        }
    }

    /// Envia a textura da fonte para a GPU e registra o manipulador `TextureId`.
    /// Embute os glifos desta fonte dentro de uma imagem de atlas maior em `(offset_x, offset_y)`
    /// e remapeia as coordenadas UV de todos os caracteres para o espaço normalizado do atlas.
    pub fn embed_into_atlas(&mut self, atlas_image: &mut RgbaImage, offset_x: u32, offset_y: u32) {
        atlas_image.blit(&self.texture_image, offset_x, offset_y);
        let orig_w = self.texture_image.width as f32;
        let orig_h = self.texture_image.height as f32;
        let atlas_w = atlas_image.width as f32;
        let atlas_h = atlas_image.height as f32;

        if atlas_w <= 0.0 || atlas_h <= 0.0 {
            return;
        }

        for glyph in self.glyphs.values_mut() {
            let px_min_x = glyph.uv_min.0 * orig_w + offset_x as f32;
            let px_min_y = glyph.uv_min.1 * orig_h + offset_y as f32;
            let px_max_x = glyph.uv_max.0 * orig_w + offset_x as f32;
            let px_max_y = glyph.uv_max.1 * orig_h + offset_y as f32;

            glyph.uv_min = (px_min_x / atlas_w, px_min_y / atlas_h);
            glyph.uv_max = (px_max_x / atlas_w, px_max_y / atlas_h);
        }
    }

    pub fn upload(&mut self, backend: &mut Box<dyn RenderingBackend>) -> TextureId {
        let tex_id = self.texture_image.upload_to_gpu(backend);
        self.texture_id = Some(tex_id);
        tex_id
    }

    /// Calcula as dimensões (largura, altura) ocupadas pelo texto na escala fornecida.
    pub fn measure_text(&self, text: &str, scale: f32) -> (f32, f32) {
        let mut max_width: f32 = 0.0;
        let mut current_width: f32 = 0.0;
        let mut lines = 1;

        for c in text.chars() {
            if c == '\n' {
                max_width = max_width.max(current_width);
                current_width = 0.0;
                lines += 1;
            } else {
                let advance = self.glyphs.get(&c).map(|g| g.advance).unwrap_or(self.base_width);
                current_width += advance * scale;
            }
        }

        max_width = max_width.max(current_width);
        let height = lines as f32 * self.line_spacing * scale;
        (max_width, height)
    }

    /// Desenha o texto no lote `SpriteBatch` alinhado à esquerda na posição (start_x, start_y).
    pub fn draw_text(
        &self,
        batch: &mut SpriteBatch,
        text: &str,
        start_x: f32,
        start_y: f32,
        scale: f32,
        color: Color,
    ) {
        let mut cursor_x = start_x;
        let mut cursor_y = start_y;

        for c in text.chars() {
            if c == '\n' {
                cursor_x = start_x;
                cursor_y += self.line_spacing * scale;
                continue;
            }

            if let Some(glyph) = self.glyphs.get(&c) {
                let gw = glyph.width * scale;
                let gh = glyph.height * scale;
                let dest = Rect::new(cursor_x, cursor_y, gw, gh);
                batch.draw_textured_rect(dest, glyph.uv_min, glyph.uv_max, color);
                cursor_x += glyph.advance * scale;
            } else {
                cursor_x += self.base_width * scale;
            }
        }
    }
}

impl Default for BitmapFont {
    fn default() -> Self {
        Self::default_font()
    }
}

/// Mapeamento de 5 colunas verticais (bits 0..=6) para caracteres ASCII essenciais.
fn get_glyph_bitmap_5x7(c: char) -> [u8; 5] {
    match c {
        // Dígitos 0..9
        '0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1E],

        // Letras Maiúsculas A..Z
        'A' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        'B' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        'C' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        'D' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        'E' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        'F' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        'G' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        'H' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        'I' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        'J' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        'K' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        'L' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        'M' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        'N' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        'O' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        'P' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        'Q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        'R' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        'U' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        'V' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        'W' => [0x7F, 0x20, 0x18, 0x20, 0x7F],
        'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],

        // Letras Minúsculas a..z
        'a' => [0x20, 0x54, 0x54, 0x54, 0x78],
        'b' => [0x7F, 0x48, 0x44, 0x44, 0x38],
        'c' => [0x38, 0x44, 0x44, 0x44, 0x20],
        'd' => [0x38, 0x44, 0x44, 0x48, 0x7F],
        'e' => [0x38, 0x54, 0x54, 0x54, 0x18],
        'f' => [0x08, 0x7E, 0x09, 0x01, 0x02],
        'g' => [0x0C, 0x52, 0x52, 0x52, 0x3E],
        'h' => [0x7F, 0x08, 0x04, 0x04, 0x78],
        'i' => [0x00, 0x44, 0x7D, 0x40, 0x00],
        'j' => [0x20, 0x40, 0x44, 0x3D, 0x00],
        'k' => [0x7F, 0x10, 0x28, 0x44, 0x00],
        'l' => [0x00, 0x41, 0x7F, 0x40, 0x00],
        'm' => [0x7C, 0x04, 0x18, 0x04, 0x78],
        'n' => [0x7C, 0x08, 0x04, 0x04, 0x78],
        'o' => [0x38, 0x44, 0x44, 0x44, 0x38],
        'p' => [0x7C, 0x14, 0x14, 0x14, 0x08],
        'q' => [0x08, 0x14, 0x14, 0x18, 0x7C],
        'r' => [0x7C, 0x08, 0x04, 0x04, 0x08],
        's' => [0x48, 0x54, 0x54, 0x54, 0x20],
        't' => [0x04, 0x3F, 0x44, 0x40, 0x20],
        'u' => [0x3C, 0x40, 0x40, 0x20, 0x7C],
        'v' => [0x1C, 0x20, 0x40, 0x20, 0x1C],
        'w' => [0x3C, 0x40, 0x30, 0x40, 0x3C],
        'x' => [0x44, 0x28, 0x10, 0x28, 0x44],
        'y' => [0x0C, 0x50, 0x50, 0x50, 0x3C],
        'z' => [0x44, 0x64, 0x54, 0x4C, 0x44],

        // Pontuação e Símbolos
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        '!' => [0x00, 0x00, 0x5F, 0x00, 0x00],
        '"' => [0x00, 0x07, 0x00, 0x07, 0x00],
        '#' => [0x14, 0x7F, 0x14, 0x7F, 0x14],
        '$' => [0x24, 0x2A, 0x7F, 0x2A, 0x12],
        '%' => [0x23, 0x13, 0x08, 0x64, 0x62],
        '&' => [0x36, 0x49, 0x55, 0x22, 0x50],
        '\'' => [0x00, 0x05, 0x03, 0x00, 0x00],
        '(' => [0x00, 0x1C, 0x22, 0x41, 0x00],
        ')' => [0x00, 0x41, 0x22, 0x1C, 0x00],
        '*' => [0x14, 0x08, 0x3E, 0x08, 0x14],
        '+' => [0x08, 0x08, 0x3E, 0x08, 0x08],
        ',' => [0x00, 0x50, 0x30, 0x00, 0x00],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        '.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        '/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        ':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        ';' => [0x00, 0x56, 0x36, 0x00, 0x00],
        '<' => [0x08, 0x14, 0x22, 0x41, 0x00],
        '=' => [0x14, 0x14, 0x14, 0x14, 0x14],
        '>' => [0x00, 0x41, 0x22, 0x14, 0x08],
        '?' => [0x02, 0x01, 0x51, 0x09, 0x06],

        _ => [0x7F, 0x41, 0x41, 0x41, 0x7F], // Fallback: Retângulo oco
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_generation_and_metrics() {
        let font = BitmapFont::default_font();
        assert_eq!(font.glyphs.len(), 95);
        assert!(font.glyphs.contains_key(&'A'));
        assert!(font.glyphs.contains_key(&'0'));
        assert!(font.glyphs.contains_key(&':'));

        let (w, h) = font.measure_text("Ouro: 500", 2.0);
        // "Ouro: 500" = 9 caracteres * 6px * 2.0 = 108px
        assert_eq!(w, 108.0);
        assert_eq!(h, 20.0);
    }

    #[test]
    fn test_font_draw_to_batch() {
        let font = BitmapFont::default_font();
        let mut batch = SpriteBatch::new(100);

        font.draw_text(&mut batch, "Bestigard v1.0", 10.0, 10.0, 1.0, Color::WHITE);
        // "Bestigard v1.0" = 14 caracteres = 14 quads no batch
        assert_eq!(batch.quad_count(), 14);
    }
}
