//! Abstração unificada de eventos de toque, mouse e gestos (Tap, Drag, Pinch-to-Zoom) em Safe Rust.
#![forbid(unsafe_code)]

use crate::camera::Camera2D;
use crate::geometry::Point;
use miniquad::{MouseButton, TouchPhase};
use std::collections::HashMap;

/// Gestos e eventos de entrada de alto nível unificados entre mobile e desktop.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Ponteiro pressionado (dedo tocou a tela ou botão do mouse pressionado).
    PointerDown { id: u64, pos: Point },
    /// Ponteiro liberado.
    PointerUp { id: u64, pos: Point },
    /// Movimento contínuo do ponteiro.
    PointerMove { id: u64, pos: Point, delta: Point },
    /// Toque rápido no mesmo ponto sem deslocamento significativo (clique/tap).
    Tap { pos: Point },
    /// Arraste contínuo para movimentação da câmera (pan).
    Pan { delta: Point, current_pos: Point },
    /// Gesto de pinça com dois dedos para zoom em touchscreen.
    PinchZoom { factor: f32, focus: Point },
    /// Zoom via roda de scroll do mouse no desktop.
    ScrollZoom { delta: f32, focus: Point },
}

/// Estado individual de um dedo em contato com a tela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveTouch {
    pub id: u64,
    pub start_pos: Point,
    pub current_pos: Point,
    pub last_pos: Point,
    pub start_time: f64,
    pub moved: bool,
}

/// Gerenciador unificado de entradas que converte eventos crus em gestos refinados.
#[derive(Debug, Clone)]
pub struct InputManager {
    /// Toques ativos em telas capacitivas (indexados pelo ID de contato da plataforma).
    active_touches: HashMap<u64, ActiveTouch>,
    /// Distância anterior entre os dois dedos do gesto de pinça (para cálculo do fator diferencial).
    last_pinch_distance: Option<f32>,
    /// Posição atual do mouse no desktop.
    mouse_pos: Point,
    /// Última posição do mouse.
    last_mouse_pos: Point,
    /// Indica se o botão principal do mouse está pressionado.
    mouse_down: bool,
    /// Posição onde o mouse foi pressionado pela última vez.
    mouse_down_pos: Point,
    /// Se o ponteiro do mouse se moveu além do limiar de clique.
    mouse_moved: bool,
    /// Limiar em pixels para diferenciar um toque estático (*tap*) de um arraste (*pan*).
    tap_threshold_pixels: f32,
    /// Eventos de gesto gerados durante o quadro atual.
    events: Vec<InputEvent>,
    /// Tempo do frame em segundos (base de simulação ou relógio real).
    current_time: f64,
}

impl InputManager {
    pub const DEFAULT_TAP_THRESHOLD: f32 = 12.0;

    pub fn new() -> Self {
        Self {
            active_touches: HashMap::new(),
            last_pinch_distance: None,
            mouse_pos: Point::ZERO,
            last_mouse_pos: Point::ZERO,
            mouse_down: false,
            mouse_down_pos: Point::ZERO,
            mouse_moved: false,
            tap_threshold_pixels: Self::DEFAULT_TAP_THRESHOLD,
            events: Vec::with_capacity(32),
            current_time: 0.0,
        }
    }

    /// Atualiza o relógio interno do gerenciador.
    pub fn update_time(&mut self, time_seconds: f64) {
        self.current_time = time_seconds;
    }

    /// Eventos gerados no frame atual.
    pub fn events(&self) -> &[InputEvent] {
        &self.events
    }

    /// Limpa a lista de eventos no início de cada frame.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    // ==========================================
    // TRATAMENTO DE TOUCHSCREEN (MOBILE)
    // ==========================================

    /// Processa eventos nativos de toque capacitivo do Miniquad.
    pub fn on_touch(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        let pos = Point::new(x, y);

        match phase {
            TouchPhase::Started => {
                let touch = ActiveTouch {
                    id,
                    start_pos: pos,
                    current_pos: pos,
                    last_pos: pos,
                    start_time: self.current_time,
                    moved: false,
                };
                self.active_touches.insert(id, touch);
                self.events.push(InputEvent::PointerDown { id, pos });

                // Se agora há exatamente 2 dedos, inicializa medição de pinça
                if self.active_touches.len() == 2 {
                    self.last_pinch_distance = Some(self.calculate_pinch_distance());
                }
            }

            TouchPhase::Moved => {
                let num_touches = self.active_touches.len();
                if let Some(touch) = self.active_touches.get_mut(&id) {
                    let delta = Point::new(pos.x - touch.current_pos.x, pos.y - touch.current_pos.y);
                    touch.last_pos = touch.current_pos;
                    touch.current_pos = pos;

                    let total_dx = pos.x - touch.start_pos.x;
                    let total_dy = pos.y - touch.start_pos.y;
                    let dist_sq = total_dx * total_dx + total_dy * total_dy;
                    if dist_sq > self.tap_threshold_pixels * self.tap_threshold_pixels {
                        touch.moved = true;
                    }

                    self.events.push(InputEvent::PointerMove { id, pos, delta });

                    // Gesto de 1 dedo = Pan (arraste da ilha)
                    if num_touches == 1 && touch.moved {
                        self.events.push(InputEvent::Pan {
                            delta,
                            current_pos: pos,
                        });
                    }
                }

                // Gesto de 2 dedos = Pinch-to-zoom (pinça capacitiva)
                if self.active_touches.len() == 2 {
                    let new_dist = self.calculate_pinch_distance();
                    if let Some(last_dist) = self.last_pinch_distance {
                        if last_dist > 1.0 {
                            let factor = new_dist / last_dist;
                            let focus = self.calculate_pinch_center();
                            self.events.push(InputEvent::PinchZoom { factor, focus });
                        }
                    }
                    self.last_pinch_distance = Some(new_dist);
                }
            }

            TouchPhase::Ended => {
                if let Some(touch) = self.active_touches.remove(&id) {
                    self.events.push(InputEvent::PointerUp { id, pos });

                    // Se não houve arraste e foi um toque breve com 1 dedo -> Tap
                    if !touch.moved && self.active_touches.is_empty() {
                        self.events.push(InputEvent::Tap { pos: touch.start_pos });
                    }
                }
                if self.active_touches.len() < 2 {
                    self.last_pinch_distance = None;
                }
            }

            TouchPhase::Cancelled => {
                self.active_touches.remove(&id);
                if self.active_touches.len() < 2 {
                    self.last_pinch_distance = None;
                }
            }
        }
    }

    fn calculate_pinch_distance(&self) -> f32 {
        let mut pts = self.active_touches.values().map(|t| t.current_pos);
        if let (Some(p1), Some(p2)) = (pts.next(), pts.next()) {
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            (dx * dx + dy * dy).sqrt()
        } else {
            0.0
        }
    }

    fn calculate_pinch_center(&self) -> Point {
        let mut pts = self.active_touches.values().map(|t| t.current_pos);
        if let (Some(p1), Some(p2)) = (pts.next(), pts.next()) {
            Point::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5)
        } else {
            Point::ZERO
        }
    }

    // ==========================================
    // TRATAMENTO DE MOUSE (DESKTOP)
    // ==========================================

    /// Processa clique de botão do mouse.
    pub fn on_mouse_button_down(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.mouse_down = true;
            let pos = Point::new(x, y);
            self.mouse_pos = pos;
            self.last_mouse_pos = pos;
            self.mouse_down_pos = pos;
            self.mouse_moved = false;

            self.events.push(InputEvent::PointerDown { id: 0, pos });
        }
    }

    /// Processa liberação do botão do mouse.
    pub fn on_mouse_button_up(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.mouse_down = false;
            let pos = Point::new(x, y);
            self.events.push(InputEvent::PointerUp { id: 0, pos });

            if !self.mouse_moved {
                self.events.push(InputEvent::Tap { pos });
            }
        }
    }

    /// Processa movimento do cursor do mouse.
    pub fn on_mouse_motion(&mut self, x: f32, y: f32) {
        let pos = Point::new(x, y);
        let delta = Point::new(pos.x - self.last_mouse_pos.x, pos.y - self.last_mouse_pos.y);
        self.last_mouse_pos = self.mouse_pos;
        self.mouse_pos = pos;

        if self.mouse_down {
            let total_dx = pos.x - self.mouse_down_pos.x;
            let total_dy = pos.y - self.mouse_down_pos.y;
            let dist_sq = total_dx * total_dx + total_dy * total_dy;
            if dist_sq > self.tap_threshold_pixels * self.tap_threshold_pixels {
                self.mouse_moved = true;
            }

            self.events.push(InputEvent::PointerMove { id: 0, pos, delta });
            if self.mouse_moved {
                self.events.push(InputEvent::Pan {
                    delta,
                    current_pos: pos,
                });
            }
        }
    }

    /// Processa rotação da roda do mouse (zoom no cursor).
    pub fn on_mouse_wheel(&mut self, _dx: f32, dy: f32) {
        self.events.push(InputEvent::ScrollZoom {
            delta: dy,
            focus: self.mouse_pos,
        });
    }

    // ==========================================
    // APLICAÇÃO DIRETA À CÂMERA
    // ==========================================

    /// Aplica automaticamente os gestos de pan e zoom gerados neste quadro à câmera do jogo.
    pub fn apply_to_camera(&self, camera: &mut Camera2D) {
        for ev in &self.events {
            match ev {
                InputEvent::Pan { delta, .. } => {
                    camera.pan(delta.x, delta.y);
                }
                InputEvent::PinchZoom { factor, focus } => {
                    camera.zoom_at(*factor, focus.x, focus.y);
                }
                InputEvent::ScrollZoom { delta, focus } => {
                    let factor = if *delta > 0.0 { 1.15 } else { 0.85 };
                    camera.zoom_at(factor, focus.x, focus.y);
                }
                _ => {}
            }
        }
    }

    /// Retorna a lista de cliques rápidos (*taps*) ocorridos no frame.
    pub fn taps(&self) -> Vec<Point> {
        self.events
            .iter()
            .filter_map(|e| match e {
                InputEvent::Tap { pos } => Some(*pos),
                _ => None,
            })
            .collect()
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_tap_and_drag() {
        let mut input = InputManager::new();

        // 1. Simula clique rápido sem arraste (Tap)
        input.on_mouse_button_down(MouseButton::Left, 100.0, 100.0);
        input.on_mouse_button_up(MouseButton::Left, 100.0, 100.0);

        let taps = input.taps();
        assert_eq!(taps.len(), 1);
        assert_eq!(taps[0], Point::new(100.0, 100.0));

        input.clear_events();

        // 2. Simula arraste (Pan)
        input.on_mouse_button_down(MouseButton::Left, 100.0, 100.0);
        input.on_mouse_motion(150.0, 100.0);
        input.on_mouse_button_up(MouseButton::Left, 150.0, 100.0);

        // Não deve gerar Tap após arraste
        assert!(input.taps().is_empty());
        let has_pan = input
            .events()
            .iter()
            .any(|e| matches!(e, InputEvent::Pan { .. }));
        assert!(has_pan);
    }

    #[test]
    fn test_touch_pinch_zoom() {
        let mut input = InputManager::new();

        // Dedo 1 em (100, 100) e Dedo 2 em (200, 100) -> Distância 100px
        input.on_touch(TouchPhase::Started, 1, 100.0, 100.0);
        input.on_touch(TouchPhase::Started, 2, 200.0, 100.0);

        input.clear_events();

        // Afasta os dedos para (50, 100) e (250, 100) -> Distância 200px (Fator 2.0x)
        input.on_touch(TouchPhase::Moved, 1, 50.0, 100.0);
        input.on_touch(TouchPhase::Moved, 2, 250.0, 100.0);

        let total_factor: f32 = input
            .events()
            .iter()
            .filter_map(|e| match e {
                InputEvent::PinchZoom { factor, .. } => Some(*factor),
                _ => None,
            })
            .product();

        assert!((total_factor - 2.0).abs() < 1e-2);

        let last_focus = input
            .events()
            .iter()
            .filter_map(|e| match e {
                InputEvent::PinchZoom { focus, .. } => Some(*focus),
                _ => None,
            })
            .last()
            .unwrap();
        assert_eq!(last_focus, Point::new(150.0, 100.0));
    }

    #[test]
    fn test_apply_to_camera() {
        let mut input = InputManager::new();
        let mut cam = Camera2D::new(800.0, 600.0);

        // Scroll zoom no ponto (400, 300)
        input.on_mouse_motion(400.0, 300.0);
        input.on_mouse_wheel(0.0, 1.0);
        input.apply_to_camera(&mut cam);

        assert!(cam.zoom > 1.0);
    }
}
