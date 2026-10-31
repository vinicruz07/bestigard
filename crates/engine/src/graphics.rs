//! Camada de contexto gráfico, cores, viewport responsivo e backend de janela em Safe Rust.
#![forbid(unsafe_code)]

use crate::app::{App, EngineConfig, GameState};
use crate::batch::GpuSpritePipeline;
use crate::input::InputManager;
use miniquad::conf::Conf;
use miniquad::{EventHandler, MouseButton, PassAction, RenderingBackend, TouchPhase};

/// Representação RGBA normalizada (0.0 a 1.0) para cores de renderização.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const CLEAR_SKY: Self = Self::rgb(0.18, 0.54, 0.88);
    pub const AETHER_NAVY: Self = Self::rgb(0.06, 0.08, 0.16);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Cria cor a partir de código hexadecimal (ex: "#1E90FF" ou "1E90FF").
    pub fn from_hex(hex: &str) -> Self {
        let clean = hex.trim_start_matches('#');
        if clean.len() >= 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0) as f32 / 255.0;
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0) as f32 / 255.0;
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0) as f32 / 255.0;
            let a = if clean.len() >= 8 {
                u8::from_str_radix(&clean[6..8], 16).unwrap_or(255) as f32 / 255.0
            } else {
                1.0
            };
            Self::rgba(r, g, b, a)
        } else {
            Self::WHITE
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::AETHER_NAVY
    }
}

/// Dimensões e proporções da área de exibição da tela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub aspect_ratio: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        let aspect_ratio = if height > 0.0 { width / height } else { 1.0 };
        Self {
            width,
            height,
            aspect_ratio,
        }
    }
}

/// Contexto gráfico e controle de frame buffers.
#[derive(Debug, Clone)]
pub struct GraphicsContext {
    pub clear_color: Color,
    pub viewport: Viewport,
    pub frame_count: u64,
}

impl GraphicsContext {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            clear_color: Color::default(),
            viewport: Viewport::new(width, height),
            frame_count: 0,
        }
    }

    /// Define a cor de fundo utilizada na limpeza de buffer.
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Atualiza as dimensões do viewport no redimensionamento da janela.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport = Viewport::new(width, height);
    }

    /// Executa a limpeza do buffer de tela padrão com a cor configurada usando o backend seguro do miniquad.
    pub fn clear(&mut self, backend: &mut Box<dyn RenderingBackend>) {
        let pass_action = PassAction::Clear {
            color: Some((
                self.clear_color.r,
                self.clear_color.g,
                self.clear_color.b,
                self.clear_color.a,
            )),
            depth: None,
            stencil: None,
        };

        backend.begin_default_pass(pass_action);
        backend.end_render_pass();
        self.frame_count += 1;
    }
}

/// Adaptador multiplataforma de janela e eventos (Safe Rust EventHandler).
pub struct WindowRunner<S: GameState> {
    pub app: App<S>,
    pub gfx: GraphicsContext,
    pub input: InputManager,
    pub gpu_pipeline: Option<GpuSpritePipeline>,
    backend: Box<dyn RenderingBackend>,
    last_frame_time: f64,
}

impl<S: GameState + 'static> WindowRunner<S> {
    pub fn new(config: EngineConfig, state: S) -> Self {
        let width = config.width as f32;
        let height = config.height as f32;
        let app = App::new(config, state);
        let gfx = GraphicsContext::new(width, height);
        let mut backend = miniquad::window::new_rendering_backend();
        let gpu_pipeline = GpuSpritePipeline::new(&mut backend, 4096).ok();
        let input = InputManager::new();

        Self {
            app,
            gfx,
            input,
            gpu_pipeline,
            backend,
            last_frame_time: miniquad::date::now(),
        }
    }

    /// Inicia o backend de janela e o loop gráfico com a configuração fornecida.
    pub fn run(config: EngineConfig, state: S) {
        let conf = Conf {
            window_title: config.title.clone(),
            window_width: config.width as i32,
            window_height: config.height as i32,
            high_dpi: true,
            fullscreen: false,
            sample_count: 1,
            ..Default::default()
        };

        miniquad::start(conf, move || Box::new(WindowRunner::new(config, state)));
    }
}

impl<S: GameState> EventHandler for WindowRunner<S> {
    fn update(&mut self) {
        let now = miniquad::date::now();
        let raw_dt = (now - self.last_frame_time).max(0.0) as f32;
        self.last_frame_time = now;
        self.input.update_time(now);

        // Notifica o jogo sobre eventos de entrada acumulados
        self.app.state_mut().on_input(&self.input);
        self.input.clear_events();

        // Avança simulação e lógica com delta time real
        self.app.step(raw_dt);
    }

    fn draw(&mut self) {
        // Limpa a tela com o clear_color configurado
        // 1. Limpa a tela com o clear_color configurado
        self.gfx.clear(&mut self.backend);

        // 2. Garante inicialização segura do pipeline de sprites GPU
        if self.gpu_pipeline.is_none() {
            self.gpu_pipeline = GpuSpritePipeline::new(&mut self.backend, 4096).ok();
        }

        // 3. Realiza upload automático do atlas para a GPU se ainda pendente
        if self.app.state().active_texture().is_none() {
            if let Some(atlas_img) = self.app.state().pending_atlas_image() {
                let tex_id = atlas_img.upload_to_gpu(&mut self.backend);
                self.app.state_mut().on_atlas_uploaded(tex_id);
            }
        }

        // 4. Submete o SpriteBatch acumulado pelo jogo à GPU
        if let Some(pipeline) = &mut self.gpu_pipeline {
            let width = self.gfx.viewport.width;
            let height = self.gfx.viewport.height;
            let texture = self.app.state().active_texture();
            if let Some(batch) = self.app.state_mut().sprite_batch_mut() {
                if !batch.is_empty() {
                    pipeline.flush(&mut self.backend, batch, texture, width, height);
                }
            }
        }
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.gfx.resize(width, height);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.input.on_mouse_motion(x, y);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.input.on_mouse_wheel(dx, dy);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.input.on_mouse_button_down(button, x, y);
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.input.on_mouse_button_up(button, x, y);
    }

    fn touch_event(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        self.input.on_touch(phase, id, x, y);
    }

    fn window_minimized_event(&mut self) {
        self.app.pause();
    }

    fn window_restored_event(&mut self) {
        self.app.resume();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex_parsing() {
        let orange = Color::from_hex("#FF4500");
        assert!((orange.r - 1.0).abs() < 1e-2);
        assert!((orange.g - 0.27).abs() < 1e-2);
        assert_eq!(orange.b, 0.0);
        assert_eq!(orange.a, 1.0);

        let white = Color::from_hex("FFFFFF");
        assert_eq!(white, Color::WHITE);
    }

    #[test]
    fn test_viewport_aspect_ratio() {
        let vp = Viewport::new(1080.0, 1920.0);
        assert!((vp.aspect_ratio - (1080.0 / 1920.0)).abs() < 1e-4);

        let mut gfx = GraphicsContext::new(800.0, 600.0);
        assert_eq!(gfx.viewport.width, 800.0);

        gfx.resize(1920.0, 1080.0);
        assert_eq!(gfx.viewport.width, 1920.0);
        assert!((gfx.viewport.aspect_ratio - (16.0 / 9.0)).abs() < 1e-4);
    }
}
