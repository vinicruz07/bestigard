//! Engine - Abstrações de ciclo de vida, gráficos, áudio e eventos para mobile/desktop.
#![forbid(unsafe_code)]

pub mod app;
pub mod atlas;
pub mod batch;
pub mod camera;
pub mod font;
pub mod geometry;
pub mod graphics;
pub mod isometric;
pub mod render;
pub mod texture;
pub mod time;

pub use app::{App, EngineConfig, GameState};
pub use atlas::{AtlasRegion, TextureAtlas};
pub use batch::{BatchStats, GpuSpritePipeline, SpriteBatch, Vertex2D};
pub use camera::Camera2D;
pub use font::{BitmapFont, Glyph};
pub use geometry::{Point, Rect};
pub use graphics::{Color, GraphicsContext, Viewport, WindowRunner};
pub use isometric::IsometricProjection;
pub use render::{RenderCommand, RenderLayer, RenderQueue};
pub use texture::RgbaImage;
pub use time::TimeStep;
