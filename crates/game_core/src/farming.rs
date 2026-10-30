//! Sistema de cultivo agrícola (fazendas de comida), alimentação e evolução de criaturas.
#![forbid(unsafe_code)]

use crate::creature::{CreatureInstance, GrowthStage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Erros relacionados ao cultivo em fazendas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FarmError {
    InsufficientGold { required: u64, available: u64 },
    FarmBusy,
    FarmNotGrowing,
    CropNotReady { remaining_secs: u64 },
    FarmIdle,
    CropNotFound { crop_id: String },
}

impl std::fmt::Display for FarmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientGold { required, available } => {
                write!(f, "Ouro insuficiente para plantar: necessário {}, disponível {}", required, available)
            }
            Self::FarmBusy => write!(f, "A fazenda já possui uma plantação em andamento"),
            Self::FarmNotGrowing => write!(f, "A fazenda não está cultivando nada no momento"),
            Self::CropNotReady { remaining_secs } => {
                write!(f, "A colheita ainda não está pronta: restam {} segundos", remaining_secs)
            }
            Self::FarmIdle => write!(f, "A fazenda está inativa, não há o que colher"),
            Self::CropNotFound { crop_id } => write!(f, "Cultura agrícola '{}' não encontrada", crop_id),
        }
    }
}

impl std::error::Error for FarmError {}

/// Erros relacionados à alimentação de criaturas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedingError {
    InsufficientFood { required: u64, available: u64 },
    MaxLevelReached { level: u32 },
}

impl std::fmt::Display for FeedingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientFood { required, available } => {
                write!(f, "Comida insuficiente para alimentar: necessário {}, disponível {}", required, available)
            }
            Self::MaxLevelReached { level } => {
                write!(f, "A criatura já atingiu o nível máximo ({})", level)
            }
        }
    }
}

impl std::error::Error for FeedingError {}

/// Tipo de semente / vegetal cultivável no jogo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CropType {
    pub id: String,
    pub name: String,
    pub gold_cost: u64,
    pub growth_time_secs: u64,
    pub food_yield: u64,
}

/// Estado atual da fazenda agrícola.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FarmState {
    Idle,
    Growing {
        crop_id: String,
        elapsed_secs: f64,
        duration_secs: u64,
        food_yield: u64,
    },
    ReadyToHarvest {
        crop_id: String,
        food_yield: u64,
    },
}

/// Instância de uma fazenda posicionada na ilha.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FarmInstance {
    pub instance_id: u64,
    pub grid_x: u32,
    pub grid_y: u32,
    pub width: u32,
    pub height: u32,
    pub state: FarmState,
}

impl FarmInstance {
    pub fn new(instance_id: u64, grid_x: u32, grid_y: u32) -> Self {
        Self {
            instance_id,
            grid_x,
            grid_y,
            width: 2,
            height: 2,
            state: FarmState::Idle,
        }
    }

    /// Inicia o plantio de uma cultura agrícola consumindo o ouro do jogador.
    pub fn plant_crop(
        &mut self,
        crop: &CropType,
        player_gold: &mut u64,
    ) -> Result<(), FarmError> {
        if self.state != FarmState::Idle {
            return Err(FarmError::FarmBusy);
        }

        if *player_gold < crop.gold_cost {
            return Err(FarmError::InsufficientGold {
                required: crop.gold_cost,
                available: *player_gold,
            });
        }

        *player_gold -= crop.gold_cost;

        self.state = FarmState::Growing {
            crop_id: crop.id.clone(),
            elapsed_secs: 0.0,
            duration_secs: crop.growth_time_secs,
            food_yield: crop.food_yield,
        };

        Ok(())
    }

    /// Atualiza o avanço do tempo de crescimento da plantação.
    pub fn update_growth(&mut self, dt_secs: f64) {
        if let FarmState::Growing {
            crop_id,
            mut elapsed_secs,
            duration_secs,
            food_yield,
        } = self.state.clone()
        {
            elapsed_secs += dt_secs;

            if elapsed_secs >= duration_secs as f64 {
                self.state = FarmState::ReadyToHarvest {
                    crop_id,
                    food_yield,
                };
            } else {
                self.state = FarmState::Growing {
                    crop_id,
                    elapsed_secs,
                    duration_secs,
                    food_yield,
                };
            }
        }
    }

    /// Colhe a comida quando a plantação estiver pronta.
    pub fn harvest(&mut self) -> Result<u64, FarmError> {
        match self.state {
            FarmState::ReadyToHarvest { food_yield, .. } => {
                self.state = FarmState::Idle;
                Ok(food_yield)
            }
            FarmState::Growing {
                elapsed_secs,
                duration_secs,
                ..
            } => {
                let remaining = (duration_secs as f64 - elapsed_secs).max(0.0).ceil() as u64;
                Err(FarmError::CropNotReady {
                    remaining_secs: remaining,
                })
            }
            FarmState::Idle => Err(FarmError::FarmIdle),
        }
    }
}

/// Catálogo de culturas agrícolas de Bestigard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CropCatalog {
    crops: HashMap<String, CropType>,
}

impl CropCatalog {
    pub fn new() -> Self {
        Self {
            crops: HashMap::new(),
        }
    }

    /// Culturas padrão calibradas para progressão balanceada sem microtransações.
    pub fn with_default_crops() -> Self {
        let mut catalog = Self::new();

        catalog.register(CropType {
            id: "wild_berry".to_string(),
            name: "Fruta Silvestre".to_string(),
            gold_cost: 30,
            growth_time_secs: 15,
            food_yield: 60,
        });

        catalog.register(CropType {
            id: "mystic_root".to_string(),
            name: "Raiz Mística".to_string(),
            gold_cost: 150,
            growth_time_secs: 60,
            food_yield: 350,
        });

        catalog.register(CropType {
            id: "arcane_wheat".to_string(),
            name: "Trigo Arcano".to_string(),
            gold_cost: 600,
            growth_time_secs: 300,
            food_yield: 1600,
        });

        catalog.register(CropType {
            id: "solar_bloom".to_string(),
            name: "Flor Solar".to_string(),
            gold_cost: 2500,
            growth_time_secs: 1200,
            food_yield: 7500,
        });

        catalog
    }

    pub fn register(&mut self, crop: CropType) {
        self.crops.insert(crop.id.clone(), crop);
    }

    pub fn get(&self, id: &str) -> Option<&CropType> {
        self.crops.get(id)
    }

    pub fn count(&self) -> usize {
        self.crops.len()
    }

    /// Retorna as culturas agrícolas cadastradas ordenadas por custo em ouro crescente.
    pub fn all_sorted(&self) -> Vec<&CropType> {
        let mut list: Vec<&CropType> = self.crops.values().collect();
        list.sort_by(|a, b| a.gold_cost.cmp(&b.gold_cost));
        list
    }
}

/// Resultado detalhado de uma ação de alimentação.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedResult {
    pub food_spent: u64,
    pub previous_level: u32,
    pub new_level: u32,
    pub leveled_up: bool,
    pub previous_stage: GrowthStage,
    pub new_stage: GrowthStage,
    pub evolved: bool,
}

/// Sistema de alimentação e evolução de criaturas.
#[derive(Debug, Clone, Default)]
pub struct FeedingSystem;

impl FeedingSystem {
    pub const MAX_LEVEL: u32 = 40;

    /// Calcula a quantidade de comida necessária para uma refeição no nível atual.
    /// Progressão suave: custo = 20 * level^1.4
    pub fn food_cost_for_level(level: u32) -> u64 {
        let lvl = level as f64;
        (20.0 * lvl.powf(1.4)).round() as u64
    }

    /// Alimenta uma criatura, concedendo XP e disparando evolução caso suba de nível.
    pub fn feed_creature(
        creature: &mut CreatureInstance,
        player_food: &mut u64,
    ) -> Result<FeedResult, FeedingError> {
        if creature.level >= Self::MAX_LEVEL {
            return Err(FeedingError::MaxLevelReached {
                level: Self::MAX_LEVEL,
            });
        }

        let cost = Self::food_cost_for_level(creature.level);

        if *player_food < cost {
            return Err(FeedingError::InsufficientFood {
                required: cost,
                available: *player_food,
            });
        }

        *player_food -= cost;

        let prev_level = creature.level;
        let prev_stage = creature.stage;

        // O XP concedido por refeição equivale ao valor do custo de alimento
        let leveled_up = creature.add_xp(cost);

        let new_level = creature.level;
        let new_stage = creature.stage;
        let evolved = new_stage != prev_stage;

        Ok(FeedResult {
            food_spent: cost,
            previous_level: prev_level,
            new_level,
            leveled_up,
            previous_stage: prev_stage,
            new_stage,
            evolved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_farming_cycle() {
        let catalog = CropCatalog::with_default_crops();
        let berry = catalog.get("wild_berry").unwrap();

        let mut farm = FarmInstance::new(1, 0, 0);
        let mut gold = 100;

        // Plantio com sucesso
        assert!(farm.plant_crop(berry, &mut gold).is_ok());
        assert_eq!(gold, 70); // Gastou 30 ouro

        // Tentar colher antes do tempo falha
        assert!(matches!(farm.harvest(), Err(FarmError::CropNotReady { .. })));

        // Avança o tempo até o término
        farm.update_growth(15.0);

        // Colheita com sucesso
        let food = farm.harvest().expect("Should harvest food successfully");
        assert_eq!(food, 60);
        assert_eq!(farm.state, FarmState::Idle);
    }

    #[test]
    fn test_feeding_and_evolution() {
        let mut creature = CreatureInstance::new(1, "pyro_salamander");
        assert_eq!(creature.level, 1);
        assert_eq!(creature.stage, GrowthStage::Baby);

        let mut food = 5000;

        // Alimenta repetidamente até atingir nível 4+ (evolução para Juvenile)
        let mut evolved_to_juvenile = false;
        for _ in 0..25 {
            if let Ok(res) = FeedingSystem::feed_creature(&mut creature, &mut food) {
                if res.evolved && res.new_stage == GrowthStage::Juvenile {
                    evolved_to_juvenile = true;
                    break;
                }
            }
        }


        assert!(evolved_to_juvenile);
        assert_eq!(creature.stage, GrowthStage::Juvenile);
        assert!(creature.can_breed());
    }
}
