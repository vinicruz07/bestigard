//! Obstáculos naturais da ilha (pedras, árvores, arbustos) que podem ser limpos para liberar espaço de construção.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Tipo de obstáculo decorativo que bloqueia células do grid da ilha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObstacleKind {
    Bush,
    SmallRock,
    Tree,
    LargeRock,
}

impl ObstacleKind {
    /// Nome de exibição (ASCII, compatível com a fonte bitmap embutida).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Bush => "ARBUSTO",
            Self::SmallRock => "PEDRA PEQUENA",
            Self::Tree => "ARVORE",
            Self::LargeRock => "ROCHEDO",
        }
    }

    /// Identificador do sprite correspondente em `docs/lista_de_sprites.md`.
    pub fn sprite_id(&self) -> &'static str {
        match self {
            Self::Bush => "obstacle_bush",
            Self::SmallRock => "obstacle_rock_small",
            Self::Tree => "obstacle_tree",
            Self::LargeRock => "obstacle_rock_large",
        }
    }

    /// Dimensão (largura, altura) em células do grid.
    pub fn footprint(&self) -> (u32, u32) {
        match self {
            Self::LargeRock => (2, 2),
            _ => (1, 1),
        }
    }

    /// Custo em ouro para limpar o obstáculo.
    pub fn clear_cost_gold(&self) -> u64 {
        match self {
            Self::Bush => 15,
            Self::SmallRock => 25,
            Self::Tree => 40,
            Self::LargeRock => 100,
        }
    }
}

/// Instância de obstáculo posicionada na ilha.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obstacle {
    pub instance_id: u64,
    pub kind: ObstacleKind,
    pub grid_x: u32,
    pub grid_y: u32,
}

impl Obstacle {
    pub fn new(instance_id: u64, kind: ObstacleKind, grid_x: u32, grid_y: u32) -> Self {
        Self { instance_id, kind, grid_x, grid_y }
    }

    /// Verifica se a célula `(x, y)` pertence à área ocupada pelo obstáculo.
    pub fn covers(&self, x: u32, y: u32) -> bool {
        let (w, h) = self.kind.footprint();
        x >= self.grid_x && x < self.grid_x + w && y >= self.grid_y && y < self.grid_y + h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obstacle_footprint_and_cover() {
        let rock = Obstacle::new(1, ObstacleKind::LargeRock, 4, 4);
        assert!(rock.covers(4, 4));
        assert!(rock.covers(5, 5));
        assert!(!rock.covers(6, 4));
        assert!(!rock.covers(3, 4));
        assert_eq!(ObstacleKind::Tree.footprint(), (1, 1));
        assert!(ObstacleKind::LargeRock.clear_cost_gold() > ObstacleKind::Bush.clear_cost_gold());
    }
}

