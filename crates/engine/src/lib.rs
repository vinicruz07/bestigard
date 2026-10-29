//! Engine - Abstrações de ciclo de vida, gráficos, áudio e eventos para mobile/desktop.
#![forbid(unsafe_code)]

pub mod app;
pub mod atlas;
pub mod batch;
pub mod camera;
pub mod geometry;
pub mod graphics;
pub mod isometric;
pub mod time;

pub use app::{App, EngineConfig, GameState};
pub use atlas::{AtlasRegion, TextureAtlas};
pub use batch::{BatchStats, GpuSpritePipeline, SpriteBatch, Vertex2D};
pub use camera::Camera2D;
pub use geometry::{Point, Rect};
pub use graphics::{Color, GraphicsContext, Viewport, WindowRunner};
pub use isometric::IsometricProjection;
pub use time::TimeStep;
