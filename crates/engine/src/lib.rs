//! Engine - Abstrações de ciclo de vida, gráficos, áudio e eventos para mobile/desktop.
#![forbid(unsafe_code)]

pub mod app;
pub mod atlas;
pub mod batch;
pub mod camera;
pub mod font;
pub mod geometry;
pub mod graphics;
pub mod input;
pub mod isometric;
pub mod placement;
pub mod render;
pub mod texture;
pub mod time;
pub mod ui;

pub use app::{App, EngineConfig, GameState};
pub use atlas::{AtlasRegion, TextureAtlas};
pub use batch::{BatchStats, GpuSpritePipeline, SpriteBatch, Vertex2D};
pub use camera::Camera2D;
pub use font::{BitmapFont, Glyph};
pub use geometry::{Point, Rect};
pub use graphics::{Color, GraphicsContext, Viewport, WindowRunner};
pub use input::{ActiveTouch, InputEvent, InputManager};
pub use miniquad::{MouseButton, TouchPhase};
pub use isometric::IsometricProjection;
pub use placement::{polygon_strips, PlacementCursor, PLACEMENT_INVALID_COLOR, PLACEMENT_VALID_COLOR};
pub use render::{RenderCommand, RenderLayer, RenderQueue};
pub use texture::RgbaImage;
pub use time::TimeStep;
pub use ui::{UiAction, UiButton, UiColor, UiManager, UiModal, UiProgressBar};

