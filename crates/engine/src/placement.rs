//! Cursor de posicionamento sobre o grid isométrico (modo de construção) com indicador de validade.
#![forbid(unsafe_code)]

use crate::camera::Camera2D;
use crate::geometry::{Point, Rect};
use crate::graphics::Color;
use crate::isometric::IsometricProjection;
use crate::render::{RenderCommand, RenderLayer, RenderQueue};

/// Cor do indicador quando a área é válida para construção (verde translúcido).
pub const PLACEMENT_VALID_COLOR: Color = Color::rgba(0.20, 0.90, 0.30, 0.50);
/// Cor do indicador quando a área é inválida (vermelho translúcido).
pub const PLACEMENT_INVALID_COLOR: Color = Color::rgba(0.95, 0.20, 0.20, 0.55);

/// Preenche um quadrilátero convexo com faixas horizontais finas (o batcher só desenha quads alinhados aos eixos).
pub fn polygon_strips(points: &[Point; 4], strip_height: f32) -> Vec<Rect> {
    let strip_height = strip_height.max(0.5);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    if !(max_y > min_y) {
        return Vec::new();
    }

    let count = ((max_y - min_y) / strip_height).ceil() as usize;
    let mut strips = Vec::with_capacity(count);

    for k in 0..count {
        let top = min_y + k as f32 * strip_height;
        let height = strip_height.min(max_y - top);
        let mid_y = top + height * 0.5;

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        for i in 0..4 {
            let a = points[i];
            let b = points[(i + 1) % 4];
            let (lo, hi) = if a.y <= b.y { (a, b) } else { (b, a) };
            if hi.y > lo.y && mid_y >= lo.y && mid_y <= hi.y {
                let t = (mid_y - lo.y) / (hi.y - lo.y);
                let x = lo.x + t * (hi.x - lo.x);
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }

        if max_x > min_x {
            strips.push(Rect::new(min_x, top, max_x - min_x, height));
        }
    }

    strips
}

/// Cursor de posicionamento de uma estrutura de `width` x `height` células sobre o grid da ilha.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementCursor {
    pub width: u32,
    pub height: u32,
    /// Célula do canto superior (origem) da área; pode ser negativa enquanto o cursor sai da ilha.
    pub origin_x: i32,
    pub origin_y: i32,
    pub valid: bool,
}

impl PlacementCursor {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, origin_x: 0, origin_y: 0, valid: false }
    }

    /// Centraliza a área sobre a célula tocada.
    pub fn center_on_cell(&mut self, cell: (i32, i32)) {
        self.origin_x = cell.0 - (self.width / 2) as i32;
        self.origin_y = cell.1 - (self.height / 2) as i32;
    }

    pub fn set_valid(&mut self, valid: bool) {
        self.valid = valid;
    }

    /// Origem como coordenadas de grid sem sinal; `None` se alguma componente é negativa.
    pub fn origin_unsigned(&self) -> Option<(u32, u32)> {
        if self.origin_x < 0 || self.origin_y < 0 {
            None
        } else {
            Some((self.origin_x as u32, self.origin_y as u32))
        }
    }

    /// Vértices do losango da área em coordenadas de tela (topo, direita, base, esquerda).
    pub fn screen_corners(&self, camera: &Camera2D, projection: &IsometricProjection) -> [Point; 4] {
        let x = self.origin_x as f32;
        let y = self.origin_y as f32;
        let w = self.width as f32;
        let h = self.height as f32;
        [
            camera.grid_to_screen(x, y, projection),
            camera.grid_to_screen(x + w, y, projection),
            camera.grid_to_screen(x + w, y + h, projection),
            camera.grid_to_screen(x, y + h, projection),
        ]
    }

    /// Enfileira o indicador verde/vermelho na camada de decalques do chão.
    pub fn push_overlay(
        &self,
        queue: &mut RenderQueue,
        camera: &Camera2D,
        projection: &IsometricProjection,
    ) {
        let color = if self.valid { PLACEMENT_VALID_COLOR } else { PLACEMENT_INVALID_COLOR };
        let corners = self.screen_corners(camera, projection);
        let strip_h = (2.0 * camera.zoom).max(1.0);
        for rect in polygon_strips(&corners, strip_h) {
            queue.push(RenderCommand::colored_rect(RenderLayer::GroundDecal, 1.0, rect, color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_on_cell_and_unsigned_origin() {
        let mut cursor = PlacementCursor::new(3, 3);
        cursor.center_on_cell((5, 5));
        assert_eq!((cursor.origin_x, cursor.origin_y), (4, 4));
        assert_eq!(cursor.origin_unsigned(), Some((4, 4)));

        cursor.center_on_cell((0, 0));
        assert_eq!(cursor.origin_unsigned(), None);

        let mut farm = PlacementCursor::new(2, 2);
        farm.center_on_cell((6, 2));
        assert_eq!((farm.origin_x, farm.origin_y), (5, 1));
    }

    #[test]
    fn test_polygon_strips_cover_diamond_area() {
        // Losango 128x64 -> área = 4096
        let diamond = [
            Point::new(64.0, 0.0),
            Point::new(128.0, 32.0),
            Point::new(64.0, 64.0),
            Point::new(0.0, 32.0),
        ];
        let area: f32 = polygon_strips(&diamond, 1.0).iter().map(|r| r.w * r.h).sum();
        assert!((area - 4096.0).abs() / 4096.0 < 0.03, "área das faixas = {}", area);
    }

    #[test]
    fn test_overlay_color_follows_validity() {
        let camera = Camera2D::new(1080.0, 1920.0);
        let projection = IsometricProjection::DEFAULT;
        let mut queue = RenderQueue::new();

        let mut cursor = PlacementCursor::new(2, 2);
        cursor.center_on_cell((4, 4));
        cursor.set_valid(true);
        cursor.push_overlay(&mut queue, &camera, &projection);
        let valid_cmds = queue.len();
        assert!(valid_cmds > 0);

        cursor.set_valid(false);
        cursor.push_overlay(&mut queue, &camera, &projection);
        assert_eq!(queue.len(), valid_cmds * 2);
    }
}

