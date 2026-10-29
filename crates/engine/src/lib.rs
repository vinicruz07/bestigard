//! Engine - Abstrações de ciclo de vida, gráficos, áudio e eventos para mobile/desktop.
#![forbid(unsafe_code)]

/// Trait base para o estado de jogo desacoplado de renderizador.
pub trait GameState {
    fn update(&mut self, dt: f32);
    fn render(&mut self);
}

/// Configuração básica da janela e ciclo de vida da engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Bestigard".to_string(),
            width: 1080,
            height: 1920,
        }
    }
}
