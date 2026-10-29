//! Projeção isométrica 2:1 e conversões de coordenadas de grid/mundo/tela em Safe Rust.
#![forbid(unsafe_code)]

use crate::geometry::{Point, Rect};
use serde::{Deserialize, Serialize};

/// Configuração de proporção e dimensões de tiles para projeção isométrica padrão (2:1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IsometricProjection {
    pub tile_width: f32,
    pub tile_height: f32,
}

impl IsometricProjection {
    /// Resolução padrão recomendada (128x64 px).
    pub const DEFAULT: Self = Self::new(128.0, 64.0);

    pub const fn new(tile_width: f32, tile_height: f32) -> Self {
        Self {
            tile_width,
            tile_height,
        }
    }

    /// Converte coordenadas de célula do grid (gx, gy) para a posição (x, y) do topo do losango isométrico no mundo.
    #[inline]
    pub fn grid_to_world(&self, gx: f32, gy: f32) -> Point {
        let half_w = self.tile_width * 0.5;
        let half_h = self.tile_height * 0.5;
        Point::new((gx - gy) * half_w, (gx + gy) * half_h)
    }

    /// Converte coordenadas de mundo (wx, wy) para coordenadas fracionárias de grid (gx, gy).
    #[inline]
    pub fn world_to_grid(&self, wx: f32, wy: f32) -> Point {
        let gx = (wx / self.tile_width) + (wy / self.tile_height);
        let gy = (wy / self.tile_height) - (wx / self.tile_width);
        Point::new(gx, gy)
    }

    /// Converte coordenadas contínuas de mundo diretamente para a célula inteira do grid correspondente.
    #[inline]
    pub fn world_to_cell(&self, wx: f32, wy: f32) -> (i32, i32) {
        let p = self.world_to_grid(wx, wy);
        (p.x.floor() as i32, p.y.floor() as i32)
    }

    /// Retorna o retângulo delimitador (*bounding box*) em espaço de mundo ocupado pelo tile do grid.
    pub fn tile_bounding_rect(&self, gx: i32, gy: i32) -> Rect {
        let origin = self.grid_to_world(gx as f32, gy as f32);
        // O losango se estende de -half_w a +half_w no eixo X e de 0 a tile_height no eixo Y
        let half_w = self.tile_width * 0.5;
        Rect::new(origin.x - half_w, origin.y, self.tile_width, self.tile_height)
    }

    /// Retorna as 4 pontas do losango isométrico de um tile no mundo:
    /// [Norte (topo), Leste (direita), Sul (baixo), Oeste (esquerda)].
    pub fn tile_diamond_points(&self, gx: i32, gy: i32) -> [Point; 4] {
        let top = self.grid_to_world(gx as f32, gy as f32);
        let half_w = self.tile_width * 0.5;
        let half_h = self.tile_height * 0.5;

        [
            top,                                           // Norte
            Point::new(top.x + half_w, top.y + half_h),    // Leste
            Point::new(top.x, top.y + self.tile_height),   // Sul
            Point::new(top.x - half_w, top.y + half_h),    // Oeste
        ]
    }

    /// Calcula a distância em passos de Manhattan entre duas células do grid.
    pub fn manhattan_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> u32 {
        ((x1 - x2).abs() + (y1 - y2).abs()) as u32
    }

    /// Calcula a distância diagonal/Chebyshev entre duas células do grid.
    pub fn chebyshev_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> u32 {
        (x1 - x2).abs().max((y1 - y2).abs()) as u32
    }
}

impl Default for IsometricProjection {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isometric_projection_roundtrip() {
        let proj = IsometricProjection::new(128.0, 64.0);

        // Testa célula central (0, 0)
        let world_origin = proj.grid_to_world(0.0, 0.0);
        assert_eq!(world_origin, Point::new(0.0, 0.0));
        let grid_back = proj.world_to_grid(world_origin.x, world_origin.y);
        assert!((grid_back.x - 0.0).abs() < 1e-4);
        assert!((grid_back.y - 0.0).abs() < 1e-4);

        // Testa célula arbitrária (4, 2)
        let world_pos = proj.grid_to_world(4.0, 2.0);
        // (4 - 2) * 64 = 128, (4 + 2) * 32 = 192
        assert_eq!(world_pos, Point::new(128.0, 192.0));

        let cell = proj.world_to_cell(128.0, 192.0);
        assert_eq!(cell, (4, 2));

        // Ponto ligeiramente deslocado dentro do mesmo losango
        let cell_offset = proj.world_to_cell(128.0, 195.0);
        assert_eq!(cell_offset, (4, 2));
    }

    #[test]
    fn test_tile_diamond_points() {
        let proj = IsometricProjection::new(128.0, 64.0);
        let diamond = proj.tile_diamond_points(0, 0);

        assert_eq!(diamond[0], Point::new(0.0, 0.0));
        assert_eq!(diamond[1], Point::new(64.0, 32.0));
        assert_eq!(diamond[2], Point::new(0.0, 64.0));
        assert_eq!(diamond[3], Point::new(-64.0, 32.0));
    }

    #[test]
    fn test_grid_distances() {
        assert_eq!(IsometricProjection::manhattan_distance(2, 3, 5, 7), 3 + 4);
        assert_eq!(IsometricProjection::chebyshev_distance(2, 3, 5, 7), 4);
    }
}
