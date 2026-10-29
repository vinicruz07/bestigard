//! Sistema de câmera 2D para visualização isométrica, pan, zoom e frustum culling em Safe Rust.
#![forbid(unsafe_code)]

use crate::geometry::{Point, Rect};
use crate::isometric::IsometricProjection;
use serde::{Deserialize, Serialize};

/// Câmera 2D com suporte a translação suave (pan), zoom proporcional e limites de mundo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera2D {
    /// Ponto do mundo onde o centro da tela está focado.
    pub position: Point,
    /// Fator de ampliação atual (1.0 = 100%, 2.0 = 200%, etc.).
    pub zoom: f32,
    /// Limite mínimo de zoom permitido.
    pub min_zoom: f32,
    /// Limite máximo de zoom permitido.
    pub max_zoom: f32,
    /// Largura da área de exibição da tela em pixels.
    pub viewport_width: f32,
    /// Altura da área de exibição da tela em pixels.
    pub viewport_height: f32,
    /// Limite opcional que restringe a navegação da câmera no espaço de mundo.
    pub world_bounds: Option<Rect>,
}

impl Camera2D {
    pub const DEFAULT_MIN_ZOOM: f32 = 0.35;
    pub const DEFAULT_MAX_ZOOM: f32 = 3.0;

    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            position: Point::new(0.0, 0.0),
            zoom: 1.0,
            min_zoom: Self::DEFAULT_MIN_ZOOM,
            max_zoom: Self::DEFAULT_MAX_ZOOM,
            viewport_width,
            viewport_height,
            world_bounds: None,
        }
    }

    /// Atualiza as dimensões da viewport no redimensionamento da janela do jogo.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Converte um ponto de tela (em pixels) para a coordenada correspondente no espaço de mundo.
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> Point {
        let half_w = self.viewport_width * 0.5;
        let half_h = self.viewport_height * 0.5;

        let world_x = (screen_x - half_w) / self.zoom + self.position.x;
        let world_y = (screen_y - half_h) / self.zoom + self.position.y;
        Point::new(world_x, world_y)
    }

    /// Converte uma coordenada de mundo para a posição final de renderização na tela.
    pub fn world_to_screen(&self, world_x: f32, world_y: f32) -> Point {
        let half_w = self.viewport_width * 0.5;
        let half_h = self.viewport_height * 0.5;

        let screen_x = (world_x - self.position.x) * self.zoom + half_w;
        let screen_y = (world_y - self.position.y) * self.zoom + half_h;
        Point::new(screen_x, screen_y)
    }

    /// Converte um clique/toque de tela diretamente para a célula inteira do grid isométrico.
    pub fn screen_to_grid_cell(
        &self,
        screen_x: f32,
        screen_y: f32,
        proj: &IsometricProjection,
    ) -> (i32, i32) {
        let world = self.screen_to_world(screen_x, screen_y);
        proj.world_to_cell(world.x, world.y)
    }

    /// Converte uma coordenada fracionária ou inteira de grid para a posição na tela.
    pub fn grid_to_screen(&self, gx: f32, gy: f32, proj: &IsometricProjection) -> Point {
        let world = proj.grid_to_world(gx, gy);
        self.world_to_screen(world.x, world.y)
    }

    /// Desloca a câmera em resposta ao arraste do mouse ou toque na tela (pan).
    /// Os parâmetros `screen_dx` e `screen_dy` são o delta de movimento na tela.
    pub fn pan(&mut self, screen_dx: f32, screen_dy: f32) {
        let world_dx = -screen_dx / self.zoom;
        let world_dy = -screen_dy / self.zoom;
        self.set_position(Point::new(
            self.position.x + world_dx,
            self.position.y + world_dy,
        ));
    }

    /// Ajusta a posição da câmera, respeitando os limites de mundo configurados.
    pub fn set_position(&mut self, target: Point) {
        if let Some(bounds) = self.world_bounds {
            let clamped_x = target.x.clamp(bounds.x, bounds.x + bounds.w);
            let clamped_y = target.y.clamp(bounds.y, bounds.y + bounds.h);
            self.position = Point::new(clamped_x, clamped_y);
        } else {
            self.position = target;
        }
    }

    /// Define o nível de zoom diretamente, respeitando limites mínimo e máximo.
    pub fn set_zoom(&mut self, new_zoom: f32) {
        self.zoom = new_zoom.clamp(self.min_zoom, self.max_zoom);
    }

    /// Executa zoom proporcional centrado em um ponto focal da tela (toque de pinça ou roda do mouse).
    pub fn zoom_at(&mut self, factor: f32, screen_focus_x: f32, screen_focus_y: f32) {
        let world_before = self.screen_to_world(screen_focus_x, screen_focus_y);

        let target_zoom = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);
        self.zoom = target_zoom;

        let world_after = self.screen_to_world(screen_focus_x, screen_focus_y);

        // Desloca a câmera para manter o ponto de foco sob o cursor/dedo estático
        let adjust_x = world_before.x - world_after.x;
        let adjust_y = world_before.y - world_after.y;
        self.set_position(Point::new(
            self.position.x + adjust_x,
            self.position.y + adjust_y,
        ));
    }

    /// Interpola suavemente a posição da câmera em direção a um alvo (lerp).
    pub fn lerp_to(&mut self, target: Point, alpha: f32) {
        let t = alpha.clamp(0.0, 1.0);
        let next_x = self.position.x + (target.x - self.position.x) * t;
        let next_y = self.position.y + (target.y - self.position.y) * t;
        self.set_position(Point::new(next_x, next_y));
    }

    /// Retorna a área retangular do mundo visível atualmente na viewport (Frustum Bounding Box).
    pub fn visible_world_bounds(&self) -> Rect {
        let top_left = self.screen_to_world(0.0, 0.0);
        let bottom_right = self.screen_to_world(self.viewport_width, self.viewport_height);

        Rect::new(
            top_left.x,
            top_left.y,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
        )
    }

    /// Verifica se um objeto delimitado por um retângulo de mundo está visível na tela (Frustum Culling).
    pub fn is_visible(&self, bounds: &Rect) -> bool {
        self.visible_world_bounds().intersects(bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_screen_to_world_roundtrip() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.set_position(Point::new(100.0, 200.0));
        cam.set_zoom(1.5);

        let screen_center = Point::new(400.0, 300.0);
        let world_point = cam.screen_to_world(screen_center.x, screen_center.y);
        assert_eq!(world_point, Point::new(100.0, 200.0));

        let back_to_screen = cam.world_to_screen(world_point.x, world_point.y);
        assert!((back_to_screen.x - screen_center.x).abs() < 1e-4);
        assert!((back_to_screen.y - screen_center.y).abs() < 1e-4);
    }

    #[test]
    fn test_camera_pan() {
        let mut cam = Camera2D::new(1000.0, 1000.0);
        cam.set_zoom(1.0);
        assert_eq!(cam.position, Point::new(0.0, 0.0));

        // Arrastar tela 100px para a esquerda move a câmera para a direita
        cam.pan(-100.0, 0.0);
        assert_eq!(cam.position, Point::new(100.0, 0.0));
    }

    #[test]
    fn test_camera_zoom_at_focus_invariance() {
        let mut cam = Camera2D::new(1080.0, 1920.0);
        cam.set_position(Point::new(500.0, 500.0));

        let focus = (300.0, 400.0);
        let world_before = cam.screen_to_world(focus.0, focus.1);

        // Aplica zoom in de 1.5x centrado em (300, 400)
        cam.zoom_at(1.5, focus.0, focus.1);

        let world_after = cam.screen_to_world(focus.0, focus.1);

        // O ponto sob o foco deve permanecer inalterado após o zoom
        assert!((world_before.x - world_after.x).abs() < 1e-3);
        assert!((world_before.y - world_after.y).abs() < 1e-3);
    }

    #[test]
    fn test_frustum_culling_visibility() {
        let cam = Camera2D::new(800.0, 600.0);
        // Centro em (0, 0), zoom 1.0 -> visível de (-400, -300) a (400, 300)
        let inside = Rect::new(50.0, 50.0, 32.0, 32.0);
        let outside = Rect::new(1000.0, 1000.0, 32.0, 32.0);

        assert!(cam.is_visible(&inside));
        assert!(!cam.is_visible(&outside));
    }
}
