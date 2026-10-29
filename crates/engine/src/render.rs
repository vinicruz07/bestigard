//! Ordenação de profundidade isométrica (Y-Sorting) e renderização por camadas em Safe Rust.
#![forbid(unsafe_code)]

use crate::atlas::AtlasRegion;
use crate::batch::SpriteBatch;
use crate::geometry::Rect;
use crate::graphics::Color;
use serde::{Deserialize, Serialize};

/// Camadas visuais de renderização do jogo ordenadas da mais profunda para a mais superficial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RenderLayer {
    /// Terreno base (tiles de grama, terra, pedra, água, abismos de penhasco).
    Ground = 0,
    /// Decalques colados ao solo (sombras projetadas, caminhos, círculos de seleção).
    GroundDecal = 1,
    /// Objetos com volume vertical ordenados por profundidade isométrica (Habitats, Fazendas, Criaturas, Árvores).
    Entities = 2,
    /// Elementos suspensos acima de entidades (copas altas, nuvens móveis, efeitos climáticos).
    Overhead = 3,
    /// Interface de usuário e HUD estático (barras de vida, botões, modais).
    Ui = 4,
}

/// Comando individual de desenho enfileirado para ordenação de profundidade antes do envio à GPU.
#[derive(Debug, Clone)]
pub struct RenderCommand {
    pub layer: RenderLayer,
    /// Chave de profundidade (para a camada `Entities`, corresponde à coordenada Y da base/pé do sprite).
    pub depth: f32,
    /// Desempate secundário de profundidade para evitar Z-fighting entre objetos na mesma linha.
    pub z_offset: i16,
    pub region: Option<AtlasRegion>,
    pub dest: Rect,
    pub color: Color,
    pub origin: (f32, f32),
    pub rotation: f32,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl RenderCommand {
    /// Cria comando de desenho para sprite texturizado.
    pub fn sprite(
        layer: RenderLayer,
        depth: f32,
        region: AtlasRegion,
        dest: Rect,
        color: Color,
    ) -> Self {
        Self {
            layer,
            depth,
            z_offset: 0,
            region: Some(region),
            dest,
            color,
            origin: (0.0, 0.0),
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
        }
    }

    /// Cria comando para retângulo de cor sólida (UI, sombras, seletores).
    pub fn colored_rect(layer: RenderLayer, depth: f32, dest: Rect, color: Color) -> Self {
        Self {
            layer,
            depth,
            z_offset: 0,
            region: None,
            dest,
            color,
            origin: (0.0, 0.0),
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
        }
    }

    /// Define desempate secundário de Z-Index.
    pub fn with_z_offset(mut self, offset: i16) -> Self {
        self.z_offset = offset;
        self
    }

    /// Define transformações completas (rotação, pivô e espelhamento).
    pub fn with_transform(
        mut self,
        origin: (f32, f32),
        rotation: f32,
        flip_x: bool,
        flip_y: bool,
    ) -> Self {
        self.origin = origin;
        self.rotation = rotation;
        self.flip_x = flip_x;
        self.flip_y = flip_y;
        self
    }
}

/// Fila de renderização por camadas com Y-Sorting automático para mundos isométricos.
#[derive(Debug, Default, Clone)]
pub struct RenderQueue {
    commands: Vec<RenderCommand>,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::with_capacity(512),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
        }
    }

    /// Adiciona um comando genérico de renderização.
    pub fn push(&mut self, cmd: RenderCommand) {
        self.commands.push(cmd);
    }

    /// Enfileira tile de terreno na camada `Ground`.
    pub fn push_ground(&mut self, region: AtlasRegion, dest: Rect, color: Color) {
        self.commands
            .push(RenderCommand::sprite(RenderLayer::Ground, 0.0, region, dest, color));
    }

    /// Enfileira decalque plano na camada `GroundDecal`.
    pub fn push_decal(&mut self, region: AtlasRegion, dest: Rect, color: Color) {
        self.commands.push(RenderCommand::sprite(
            RenderLayer::GroundDecal,
            0.0,
            region,
            dest,
            color,
        ));
    }

    /// Enfileira entidade com profundidade calculada pela coordenada Y da base (Y-Sorting).
    /// O parâmetro `foot_y` representa a base/pé do objeto no espaço de mundo.
    pub fn push_entity(
        &mut self,
        foot_y: f32,
        region: AtlasRegion,
        dest: Rect,
        color: Color,
    ) {
        self.commands
            .push(RenderCommand::sprite(RenderLayer::Entities, foot_y, region, dest, color));
    }

    /// Enfileira entidade em coordenadas de grid isométrico `(gx, gy)`.
    /// Em projeções 2:1, a profundidade isométrica natural cresce proporcionalmente a `gx + gy`.
    pub fn push_entity_grid(
        &mut self,
        gx: f32,
        gy: f32,
        footprint_offset: f32,
        region: AtlasRegion,
        dest: Rect,
        color: Color,
    ) {
        let depth = (gx + gy) + footprint_offset;
        self.commands
            .push(RenderCommand::sprite(RenderLayer::Entities, depth, region, dest, color));
    }

    /// Enfileira elemento da interface de usuário na camada `Ui`.
    pub fn push_ui(&mut self, region: AtlasRegion, dest: Rect, color: Color) {
        self.commands
            .push(RenderCommand::sprite(RenderLayer::Ui, 0.0, region, dest, color));
    }

    /// Enfileira caixa colorida sólida na interface (ex: painéis, fundos, barras de HP).
    pub fn push_ui_rect(&mut self, dest: Rect, color: Color) {
        self.commands
            .push(RenderCommand::colored_rect(RenderLayer::Ui, 0.0, dest, color));
    }

    /// Quantidade de comandos enfileirados no quadro atual.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Ordena comandos:
    /// 1º: Por `RenderLayer` (Ground -> GroundDecal -> Entities -> Overhead -> Ui).
    /// 2º: Dentro de `Entities`, por `depth` (Y-Sorting da base de menor para maior).
    /// 3º: Desempate por `z_offset`.
    pub fn sort(&mut self) {
        self.commands.sort_by(|a, b| {
            a.layer
                .cmp(&b.layer)
                .then_with(|| {
                    a.depth
                        .partial_cmp(&b.depth)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.z_offset.cmp(&b.z_offset))
        });
    }

    /// Submete todos os comandos ordenados para o `SpriteBatch` e limpa a fila.
    pub fn flush_to_batch(&mut self, batch: &mut SpriteBatch) {
        self.sort();

        for cmd in self.commands.drain(..) {
            if let Some(ref reg) = cmd.region {
                if cmd.rotation != 0.0 || cmd.flip_x || cmd.flip_y {
                    batch.draw_sprite_transformed(
                        reg,
                        cmd.dest,
                        cmd.origin,
                        cmd.rotation,
                        cmd.flip_x,
                        cmd.flip_y,
                        cmd.color,
                    );
                } else {
                    batch.draw_sprite(reg, cmd.dest, cmd.color);
                }
            } else {
                batch.draw_colored_rect(cmd.dest, cmd.color);
            }
        }
    }

    /// Limpa a fila sem submeter.
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_order() {
        let mut queue = RenderQueue::new();
        let dummy_rect = Rect::new(0.0, 0.0, 32.0, 32.0);

        // Adiciona em ordem invertida intencionalmente
        queue.push_ui_rect(dummy_rect, Color::WHITE);
        queue.push(RenderCommand::colored_rect(
            RenderLayer::GroundDecal,
            0.0,
            dummy_rect,
            Color::WHITE,
        ));
        queue.push(RenderCommand::colored_rect(
            RenderLayer::Ground,
            0.0,
            dummy_rect,
            Color::WHITE,
        ));
        queue.push(RenderCommand::colored_rect(
            RenderLayer::Entities,
            10.0,
            dummy_rect,
            Color::WHITE,
        ));

        queue.sort();

        assert_eq!(queue.commands[0].layer, RenderLayer::Ground);
        assert_eq!(queue.commands[1].layer, RenderLayer::GroundDecal);
        assert_eq!(queue.commands[2].layer, RenderLayer::Entities);
        assert_eq!(queue.commands[3].layer, RenderLayer::Ui);
    }

    #[test]
    fn test_isometric_y_sorting_for_entities() {
        let mut queue = RenderQueue::new();
        let dummy_rect = Rect::new(0.0, 0.0, 64.0, 64.0);

        // Criatura A atrás do habitat (depth menor = base mais ao norte no mundo)
        queue.push(RenderCommand::colored_rect(
            RenderLayer::Entities,
            150.0,
            dummy_rect,
            Color::WHITE,
        ));

        // Habitat intermediário
        queue.push(RenderCommand::colored_rect(
            RenderLayer::Entities,
            200.0,
            dummy_rect,
            Color::WHITE,
        ));

        // Criatura B na frente do habitat (depth maior = base mais ao sul)
        queue.push(RenderCommand::colored_rect(
            RenderLayer::Entities,
            250.0,
            dummy_rect,
            Color::WHITE,
        ));

        queue.sort();

        // Deve renderizar a Criatura A primeiro, depois o Habitat, depois a Criatura B
        assert_eq!(queue.commands[0].depth, 150.0);
        assert_eq!(queue.commands[1].depth, 200.0);
        assert_eq!(queue.commands[2].depth, 250.0);
    }

    #[test]
    fn test_flush_to_batch() {
        let mut queue = RenderQueue::new();
        let mut batch = SpriteBatch::new(10);
        let dummy_rect = Rect::new(0.0, 0.0, 50.0, 50.0);

        queue.push_ui_rect(dummy_rect, Color::WHITE);
        queue.push_ui_rect(dummy_rect, Color::BLACK);

        assert_eq!(queue.len(), 2);
        queue.flush_to_batch(&mut batch);

        assert_eq!(queue.len(), 0);
        assert_eq!(batch.quad_count(), 2);
    }
}
