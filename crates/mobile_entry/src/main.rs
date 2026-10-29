//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{EngineConfig, GameState};

pub struct BestigardGame;

impl GameState for BestigardGame {
    fn update(&mut self, _dt: f32) {}
    fn render(&mut self) {}
}

fn main() {
    let config = EngineConfig::default();
    println!(
        "Iniciando {} (core v{}) - Target: {}x{}",
        config.title,
        game_core::core_version(),
        config.width,
        config.height
    );
}
