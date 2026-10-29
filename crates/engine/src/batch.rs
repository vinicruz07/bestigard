//! Pipeline de desenho em lote (Sprite Batcher) e buffers GPU em Safe Rust.
#![forbid(unsafe_code)]

use crate::atlas::AtlasRegion;
use crate::geometry::Rect;
use crate::graphics::Color;
use miniquad::graphics::{
    BlendFactor, BlendState, BlendValue, BufferId, BufferLayout, BufferSource, BufferType,
    BufferUsage, Equation, Pipeline, PipelineParams, RenderingBackend, ShaderMeta,
    ShaderSource, TextureId, UniformBlockLayout, UniformDesc, UniformType, UniformsSource,
    VertexAttribute, VertexFormat,
};

/// Vértice 2D seguro para GPU.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex2D {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex2D {
    pub const fn new(pos: [f32; 2], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self { pos, uv, color }
    }
}

/// Métricas de desempenho do batcher durante um quadro.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BatchStats {
    pub quads: usize,
    pub vertices: usize,
    pub indices: usize,
    pub draw_calls: usize,
    pub flushes: usize,
}

/// Agrupador de quadriláteros e sprites 2D para submissão em lote único (*Draw Call único*).
#[derive(Debug, Clone)]
pub struct SpriteBatch {
    vertices: Vec<Vertex2D>,
    indices: Vec<u16>,
    capacity: usize,
    stats: BatchStats,
}

impl SpriteBatch {
    pub const DEFAULT_CAPACITY: usize = 2048;

    pub fn new(capacity: usize) -> Self {
        let max_quads = capacity.max(1);
        Self {
            vertices: Vec::with_capacity(max_quads * 4),
            indices: Vec::with_capacity(max_quads * 6),
            capacity: max_quads,
            stats: BatchStats::default(),
        }
    }

    /// Limpa buffers para o próximo lote/frame e preserva capacidade alocada.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.stats.quads = 0;
        self.stats.vertices = 0;
        self.stats.indices = 0;
    }

    /// Reinicia contadores de estatísticas de submissão (draw calls/flushes).
    pub fn reset_stats(&mut self) {
        self.clear();
        self.stats.draw_calls = 0;
        self.stats.flushes = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.stats.quads >= self.capacity
    }

    pub fn quad_count(&self) -> usize {
        self.stats.quads
    }

    pub fn vertices(&self) -> &[Vertex2D] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    pub fn stats(&self) -> BatchStats {
        self.stats
    }

    /// Adiciona 4 vértices arbitrários e 6 índices gerando 2 triângulos.
    pub fn push_quad(
        &mut self,
        points: [[f32; 2]; 4],
        uvs: [[f32; 2]; 4],
        colors: [[f32; 4]; 4],
    ) -> bool {
        if self.is_full() {
            return false;
        }

        let base_index = self.vertices.len() as u16;

        self.vertices.push(Vertex2D::new(points[0], uvs[0], colors[0]));
        self.vertices.push(Vertex2D::new(points[1], uvs[1], colors[1]));
        self.vertices.push(Vertex2D::new(points[2], uvs[2], colors[2]));
        self.vertices.push(Vertex2D::new(points[3], uvs[3], colors[3]));

        // Triângulo 1 (0, 1, 2) e Triângulo 2 (0, 2, 3)
        self.indices.push(base_index);
        self.indices.push(base_index + 1);
        self.indices.push(base_index + 2);
        self.indices.push(base_index);
        self.indices.push(base_index + 2);
        self.indices.push(base_index + 3);

        self.stats.quads += 1;
        self.stats.vertices = self.vertices.len();
        self.stats.indices = self.indices.len();
        true
    }

    /// Desenha um sprite mapeado em atlas no retângulo de destino sem rotação.
    pub fn draw_sprite(&mut self, region: &AtlasRegion, dest: Rect, color: Color) -> bool {
        let points = [
            dest.top_left().into(),
            dest.top_right().into(),
            dest.bottom_right().into(),
            dest.bottom_left().into(),
        ];
        let uvs = region.quad_uvs(false, false);
        let c = [color.r, color.g, color.b, color.a];
        self.push_quad(points, uvs, [c, c, c, c])
    }

    /// Desenha um sprite mapeado com transformações (origem de rotação, ângulo, espelhamento).
    pub fn draw_sprite_transformed(
        &mut self,
        region: &AtlasRegion,
        dest: Rect,
        origin: (f32, f32),
        rotation_rad: f32,
        flip_x: bool,
        flip_y: bool,
        color: Color,
    ) -> bool {
        let (ox, oy) = (dest.x + origin.0, dest.y + origin.1);
        let uvs = region.quad_uvs(flip_x, flip_y);

        // Offsets relativos ao pivô
        let p_offsets = [
            (-origin.0, -origin.1),
            (dest.w - origin.0, -origin.1),
            (dest.w - origin.0, dest.h - origin.1),
            (-origin.0, dest.h - origin.1),
        ];

        let cos = rotation_rad.cos();
        let sin = rotation_rad.sin();

        let mut points = [[0.0f32; 2]; 4];
        for i in 0..4 {
            let (dx, dy) = p_offsets[i];
            let rx = dx * cos - dy * sin;
            let ry = dx * sin + dy * cos;
            points[i] = [ox + rx, oy + ry];
        }

        let c = [color.r, color.g, color.b, color.a];
        self.push_quad(points, uvs, [c, c, c, c])
    }

    /// Desenha retângulo com cor sólida (usado para UI, fundos, barras de vida e placeholders).
    pub fn draw_colored_rect(&mut self, dest: Rect, color: Color) -> bool {
        let points = [
            dest.top_left().into(),
            dest.top_right().into(),
            dest.bottom_right().into(),
            dest.bottom_left().into(),
        ];
        // UV unitário para pixel branco de textura padrão
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let c = [color.r, color.g, color.b, color.a];
        self.push_quad(points, uvs, [c, c, c, c])
    }

    /// Desenha um retângulo com coordenadas UV arbitrárias (usado para texto e sub-regiões de texturas).
    pub fn draw_textured_rect(
        &mut self,
        dest: Rect,
        uv_min: (f32, f32),
        uv_max: (f32, f32),
        color: Color,
    ) -> bool {
        let points = [
            dest.top_left().into(),
            dest.top_right().into(),
            dest.bottom_right().into(),
            dest.bottom_left().into(),
        ];
        let uvs = [
            [uv_min.0, uv_min.1],
            [uv_max.0, uv_min.1],
            [uv_max.0, uv_max.1],
            [uv_min.0, uv_max.1],
        ];
        let c = [color.r, color.g, color.b, color.a];
        self.push_quad(points, uvs, [c, c, c, c])
    }
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CAPACITY)
    }
}

// Shader GLSL 100/300es para renderização 2D acelerada
const VERTEX_SHADER_SRC: &str = r#"#version 100
precision mediump float;
attribute vec2 in_pos;
attribute vec2 in_uv;
attribute vec4 in_color;

varying vec2 v_uv;
varying vec4 v_color;

uniform vec2 u_resolution;

void main() {
    // Projeção ortográfica para espaço normalizado de tela [-1.0, 1.0]
    vec2 zero_to_one = in_pos / u_resolution;
    vec2 zero_to_two = zero_to_one * 2.0;
    vec2 clip_space = zero_to_two - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
    v_uv = in_uv;
    v_color = in_color;
}
"#;

const FRAGMENT_SHADER_SRC: &str = r#"#version 100
precision mediump float;
varying vec2 v_uv;
varying vec4 v_color;

uniform sampler2D u_texture;

void main() {
    vec4 tex_color = texture2D(u_texture, v_uv);
    gl_FragColor = tex_color * v_color;
}
"#;

#[repr(C)]
struct Uniforms {
    u_resolution: [f32; 2],
}

/// Pipeline de shaders e buffers GPU para submissão do SpriteBatch via Miniquad.
pub struct GpuSpritePipeline {
    pipeline: Pipeline,
    vertex_buffer: BufferId,
    index_buffer: BufferId,
    default_white_texture: TextureId,
    capacity: usize,
}

impl GpuSpritePipeline {
    /// Inicializa os shaders, buffers dinâmicos e pipeline 2D com alpha blending ativado.
    pub fn new(backend: &mut Box<dyn RenderingBackend>, capacity: usize) -> Result<Self, String> {
        let shader_meta = ShaderMeta {
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("u_resolution", UniformType::Float2)],
            },
            images: vec!["u_texture".to_string()],
        };

        let shader_source = ShaderSource::Glsl {
            vertex: VERTEX_SHADER_SRC,
            fragment: FRAGMENT_SHADER_SRC,
        };

        let shader = backend
            .new_shader(shader_source, shader_meta)
            .map_err(|e| format!("Shader compilation failed: {:?}", e))?;

        let vertex_buffer = backend.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<Vertex2D>(capacity * 4),
        );

        let index_buffer = backend.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<u16>(capacity * 6),
        );

        let pipeline_params = PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..Default::default()
        };

        let buffer_layouts = [BufferLayout::default()];
        let vertex_attributes = [
            VertexAttribute::new("in_pos", VertexFormat::Float2),
            VertexAttribute::new("in_uv", VertexFormat::Float2),
            VertexAttribute::new("in_color", VertexFormat::Float4),
        ];

        let pipeline = backend.new_pipeline(
            &buffer_layouts,
            &vertex_attributes,
            shader,
            pipeline_params,
        );

        // Cria uma textura 1x1 branca para permitir desenhar cores sólidas sem atlas carregado
        let default_white_texture = backend.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);

        Ok(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            default_white_texture,
            capacity,
        })
    }

    /// Capacidade máxima de quads suportada pelo pipeline por lote.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Envia o lote de sprites para a GPU e executa o draw call consolidado.
    pub fn flush(
        &mut self,
        backend: &mut Box<dyn RenderingBackend>,
        batch: &mut SpriteBatch,
        texture: Option<TextureId>,
        screen_width: f32,
        screen_height: f32,
    ) {
        if batch.is_empty() {
            return;
        }

        // Upload seguro dos vértices e índices
        backend.buffer_update(
            self.vertex_buffer,
            BufferSource::slice(batch.vertices()),
        );
        backend.buffer_update(
            self.index_buffer,
            BufferSource::slice(batch.indices()),
        );

        let active_texture = texture.unwrap_or(self.default_white_texture);

        let bindings = miniquad::graphics::Bindings {
            vertex_buffers: vec![self.vertex_buffer],
            index_buffer: self.index_buffer,
            images: vec![active_texture],
        };

        let uniforms = Uniforms {
            u_resolution: [screen_width, screen_height],
        };

        backend.apply_pipeline(&self.pipeline);
        backend.apply_bindings(&bindings);
        backend.apply_uniforms(UniformsSource::table(&uniforms));

        backend.draw(0, batch.indices().len() as i32, 1);

        batch.stats.draw_calls += 1;
        batch.stats.flushes += 1;
        batch.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_batch_quad_accumulation() {
        let mut batch = SpriteBatch::new(100);
        assert!(batch.is_empty());
        assert_eq!(batch.quad_count(), 0);

        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let added = batch.draw_colored_rect(r, Color::WHITE);
        assert!(added);
        assert_eq!(batch.quad_count(), 1);
        assert_eq!(batch.vertices().len(), 4);
        assert_eq!(batch.indices().len(), 6);

        // Adiciona mais um
        batch.draw_colored_rect(Rect::new(50.0, 50.0, 30.0, 30.0), Color::CLEAR_SKY);
        assert_eq!(batch.quad_count(), 2);
        assert_eq!(batch.vertices().len(), 8);
        assert_eq!(batch.indices().len(), 12);

        // Limpeza
        batch.clear();
        assert!(batch.is_empty());
        assert_eq!(batch.quad_count(), 0);
    }

    #[test]
    fn test_batch_capacity_overflow_protection() {
        let mut batch = SpriteBatch::new(2);
        assert!(batch.draw_colored_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::WHITE));
        assert!(batch.draw_colored_rect(Rect::new(10.0, 0.0, 10.0, 10.0), Color::WHITE));
        assert!(batch.is_full());

        // Terceiro excede capacidade do batch
        assert!(!batch.draw_colored_rect(Rect::new(20.0, 0.0, 10.0, 10.0), Color::WHITE));
        assert_eq!(batch.quad_count(), 2);
    }

    #[test]
    fn test_sprite_rotation_and_transform() {
        let mut batch = SpriteBatch::new(10);
        let region = AtlasRegion::new("test", Rect::new(0.0, 0.0, 32.0, 32.0), 256.0, 256.0);
        let dest = Rect::new(100.0, 100.0, 32.0, 32.0);

        let ok = batch.draw_sprite_transformed(
            &region,
            dest,
            (16.0, 16.0),
            std::f32::consts::PI * 0.5,
            true,
            false,
            Color::WHITE,
        );
        assert!(ok);
        assert_eq!(batch.quad_count(), 1);
    }
}
