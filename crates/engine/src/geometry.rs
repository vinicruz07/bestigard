//! Primitivas geométricas 2D (Retângulos, Pontos, Bounding Boxes) em Safe Rust.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Ponto 2D no espaço de tela ou mundo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Retângulo 2D definido por origem (x, y) e dimensões (largura w, altura h).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Retorna as coordenadas do centro do retângulo.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// Limite superior esquerdo (min_x, min_y).
    pub fn top_left(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Limite superior direito (max_x, min_y).
    pub fn top_right(&self) -> (f32, f32) {
        (self.x + self.w, self.y)
    }

    /// Limite inferior direito (max_x, max_y).
    pub fn bottom_right(&self) -> (f32, f32) {
        (self.x + self.w, self.y + self.h)
    }

    /// Limite inferior esquerdo (min_x, max_y).
    pub fn bottom_left(&self) -> (f32, f32) {
        (self.x, self.y + self.h)
    }

    /// Verifica se um ponto 2D está contido dentro do retângulo.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    /// Verifica interseção (AABB) com outro retângulo.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Converte dimensões em pixels para coordenadas UV normalizadas [0.0, 1.0].
    pub fn uv_normalized(&self, tex_width: f32, tex_height: f32) -> (f32, f32, f32, f32) {
        if tex_width <= 0.0 || tex_height <= 0.0 {
            return (0.0, 0.0, 1.0, 1.0);
        }
        let u_min = self.x / tex_width;
        let v_min = self.y / tex_height;
        let u_max = (self.x + self.w) / tex_width;
        let v_max = (self.y + self.h) / tex_height;
        (u_min, v_min, u_max, v_max)
    }

    /// Desloca o retângulo por um vetor (dx, dy).
    pub fn translate(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    /// Escala o retângulo proporcionalmente em torno da sua origem.
    pub fn scale(&self, sx: f32, sy: f32) -> Self {
        Self::new(self.x, self.y, self.w * sx, self.h * sy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains_and_center() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.center(), (60.0, 45.0));
        assert!(r.contains(15.0, 25.0));
        assert!(r.contains(10.0, 20.0));
        assert!(r.contains(110.0, 70.0));
        assert!(!r.contains(5.0, 25.0));
        assert!(!r.contains(115.0, 25.0));
    }

    #[test]
    fn test_rect_intersection() {
        let a = Rect::new(0.0, 0.0, 50.0, 50.0);
        let b = Rect::new(25.0, 25.0, 50.0, 50.0);
        let c = Rect::new(100.0, 100.0, 20.0, 20.0);

        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_uv_normalized() {
        let r = Rect::new(0.0, 0.0, 256.0, 128.0);
        let (u_min, v_min, u_max, v_max) = r.uv_normalized(512.0, 512.0);
        assert_eq!(u_min, 0.0);
        assert_eq!(v_min, 0.0);
        assert_eq!(u_max, 0.5);
        assert_eq!(v_max, 0.25);
    }
}
