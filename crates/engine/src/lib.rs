//! Engine - Abstrações de ciclo de vida, gráficos, áudio e eventos para mobile/desktop.
#![forbid(unsafe_code)]

pub mod app;
pub mod time;

pub use app::{App, EngineConfig, GameState};
pub use time::TimeStep;
