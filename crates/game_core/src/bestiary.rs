//! Bestiário, compêndio de descobertas e sistema de marcos de recompensas sem paywall.
#![forbid(unsafe_code)]

use crate::creature::SpeciesCatalog;
use crate::element::ElementId;
use crate::incubation::IncubationSpeedModifier;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Erros relacionados a marcos e ao bestiário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BestiaryError {
    MilestoneNotFound { milestone_id: String },
    MilestoneAlreadyClaimed { milestone_id: String },
    MilestoneConditionNotMet { milestone_id: String },
}

impl std::fmt::Display for BestiaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MilestoneNotFound { milestone_id } => {
                write!(f, "Marco de recompensa '{}' não encontrado", milestone_id)
            }
            Self::MilestoneAlreadyClaimed { milestone_id } => {
                write!(f, "O marco '{}' já foi reivindicado anteriormente", milestone_id)
            }
            Self::MilestoneConditionNotMet { milestone_id } => {
                write!(f, "Requisitos para o marco '{}' ainda não foram alcançados", milestone_id)
            }
        }
    }
}

impl std::error::Error for BestiaryError {}

/// Registro de uma espécie descoberta no Bestiário do jogador.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredEntry {
    pub species_id: String,
    pub first_discovered_secs: u64,
    pub highest_level_achieved: u32,
}

/// Recompensa justa e generosa concedida ao atingir um marco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MilestoneReward {
    pub gold: u64,
    pub food: u64,
    pub speed_modifier: Option<IncubationSpeedModifier>,
}

/// Condição necessária para completar um marco do Bestiário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MilestoneCondition {
    /// Desbloquear uma quantidade total de espécies únicas.
    TotalDiscovered(usize),
    /// Obter todas as espécies específicas de uma lista (ex: todas da família de Fogo).
    SpecificSpecies(Vec<String>),
    /// Desbloquear uma quantidade de criaturas de determinado elemento.
    ElementCount {
        element_id: ElementId,
        required_count: usize,
    },
    /// Elevar uma espécie específica até determinado nível mínimo.
    SpeciesMinLevel {
        species_id: String,
        min_level: u32,
    },
}

/// Definição de um marco de conquista no compêndio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionMilestone {
    pub id: String,
    pub title: String,
    pub description: String,
    pub condition: MilestoneCondition,
    pub reward: MilestoneReward,
}

/// Livro do Bestiário que rastreia todo o progresso de coleção do jogador.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bestiary {
    pub discoveries: HashMap<String, DiscoveredEntry>,
    pub claimed_milestones: HashSet<String>,
}

impl Bestiary {
    pub fn new() -> Self {
        Self {
            discoveries: HashMap::new(),
            claimed_milestones: HashSet::new(),
        }
    }

    /// Registra a descoberta de uma nova espécie. Retorna true se for uma descoberta inédita.
    pub fn record_discovery(&mut self, species_id: &str, current_time_secs: u64) -> bool {
        if let Some(entry) = self.discoveries.get_mut(species_id) {
            if entry.highest_level_achieved == 0 {
                entry.highest_level_achieved = 1;
            }
            false
        } else {
            self.discoveries.insert(
                species_id.to_string(),
                DiscoveredEntry {
                    species_id: species_id.to_string(),
                    first_discovered_secs: current_time_secs,
                    highest_level_achieved: 1,
                },
            );
            true
        }
    }

    /// Atualiza o nível mais alto alcançado para a espécie.
    pub fn record_level(&mut self, species_id: &str, level: u32) {
        if let Some(entry) = self.discoveries.get_mut(species_id) {
            if level > entry.highest_level_achieved {
                entry.highest_level_achieved = level;
            }
        }
    }

    /// Retorna se a espécie já foi descoberta pelo jogador.
    pub fn is_discovered(&self, species_id: &str) -> bool {
        self.discoveries.contains_key(species_id)
    }

    /// Retorna se um marco específico de coleção já foi resgatado.
    pub fn is_milestone_claimed(&self, milestone_id: &str) -> bool {
        self.claimed_milestones.contains(milestone_id)
    }

    /// Total de espécies catalogadas pelo jogador.
    pub fn discovered_count(&self) -> usize {
        self.discoveries.len()
    }

    /// Porcentagem de conclusão do Bestiário (0.0 a 100.0).
    pub fn completion_percentage(&self, catalog: &SpeciesCatalog) -> f32 {
        let total = catalog.count();
        if total == 0 {
            100.0
        } else {
            (self.discoveries.len() as f32 / total as f32) * 100.0
        }
    }

    /// Avalia se a condição de um marco foi totalmente cumprida.
    pub fn is_milestone_met(
        &self,
        milestone: &CollectionMilestone,
        catalog: &SpeciesCatalog,
    ) -> bool {
        match &milestone.condition {
            MilestoneCondition::TotalDiscovered(target) => self.discoveries.len() >= *target,
            MilestoneCondition::SpecificSpecies(species_list) => {
                species_list.iter().all(|id| self.is_discovered(id))
            }
            MilestoneCondition::ElementCount {
                element_id,
                required_count,
            } => {
                let count = self
                    .discoveries
                    .keys()
                    .filter_map(|id| catalog.get(id))
                    .filter(|species| species.elements.contains(element_id))
                    .count();
                count >= *required_count
            }
            MilestoneCondition::SpeciesMinLevel {
                species_id,
                min_level,
            } => {
                if let Some(entry) = self.discoveries.get(species_id) {
                    entry.highest_level_achieved >= *min_level
                } else {
                    false
                }
            }
        }
    }

    /// Reivindica a recompensa de um marco cumprido, creditando recursos.
    pub fn claim_milestone(
        &mut self,
        milestone_id: &str,
        milestone_catalog: &MilestoneCatalog,
        catalog: &SpeciesCatalog,
        player_gold: &mut u64,
        player_food: &mut u64,
    ) -> Result<MilestoneReward, BestiaryError> {
        if self.claimed_milestones.contains(milestone_id) {
            return Err(BestiaryError::MilestoneAlreadyClaimed {
                milestone_id: milestone_id.to_string(),
            });
        }

        let milestone = milestone_catalog
            .get(milestone_id)
            .ok_or_else(|| BestiaryError::MilestoneNotFound {
                milestone_id: milestone_id.to_string(),
            })?;

        if !self.is_milestone_met(milestone, catalog) {
            return Err(BestiaryError::MilestoneConditionNotMet {
                milestone_id: milestone_id.to_string(),
            });
        }

        // Credita os recursos intrínsecos
        *player_gold += milestone.reward.gold;
        *player_food += milestone.reward.food;

        self.claimed_milestones.insert(milestone_id.to_string());

        Ok(milestone.reward.clone())
    }
}

/// Catálogo de marcos de coleção disponíveis no jogo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MilestoneCatalog {
    milestones: HashMap<String, CollectionMilestone>,
}

impl MilestoneCatalog {
    pub fn new() -> Self {
        Self {
            milestones: HashMap::new(),
        }
    }

    /// Marcos padrão com recompensas para impulsionar a progressão orgânica.
    pub fn with_default_milestones() -> Self {
        let mut catalog = Self::new();

        // Marco 1: Primeiro Passo
        catalog.register(CollectionMilestone {
            id: "first_discovery".to_string(),
            title: "Despertar das Bestas".to_string(),
            description: "Descubra sua primeira criatura no santuário.".to_string(),
            condition: MilestoneCondition::TotalDiscovered(1),
            reward: MilestoneReward {
                gold: 200,
                food: 100,
                speed_modifier: None,
            },
        });

        // Marco 2: Trio Elemental Primordial
        catalog.register(CollectionMilestone {
            id: "triad_elements".to_string(),
            title: "Tríade Elemental".to_string(),
            description: "Descubra a Salamandra Ígnea, a Serpente das Marés e o Golem Rochoso.".to_string(),
            condition: MilestoneCondition::SpecificSpecies(vec![
                "pyro_salamander".to_string(),
                "tide_serpent".to_string(),
                "stone_golem".to_string(),
            ]),
            reward: MilestoneReward {
                gold: 1500,
                food: 800,
                speed_modifier: None,
            },
        });

        // Marco 3: Mestre do Fogo (3 espécies de fogo)
        catalog.register(CollectionMilestone {
            id: "pyro_mastery".to_string(),
            title: "Linhagem das Chamas".to_string(),
            description: "Descubra 2 criaturas com afinidade de Fogo.".to_string(),
            condition: MilestoneCondition::ElementCount {
                element_id: ElementId::new("fire"),
                required_count: 2,
            },
            reward: MilestoneReward {
                gold: 1000,
                food: 600,
                speed_modifier: None,
            },
        });

        // Marco 4: Colecionador Dedicado (5 espécies)
        catalog.register(CollectionMilestone {
            id: "collector_tier1".to_string(),
            title: "Colecionador Novato".to_string(),
            description: "Registre 5 espécies diferentes no Bestiário.".to_string(),
            condition: MilestoneCondition::TotalDiscovered(5),
            reward: MilestoneReward {
                gold: 3000,
                food: 2000,
                speed_modifier: None,
            },
        });

        // Marco 5: Harmonia do Eclipse (Quimera do Eclipse)
        catalog.register(CollectionMilestone {
            id: "eclipse_discovery".to_string(),
            title: "Mistério do Crepúsculo".to_string(),
            description: "Descubra a lendária Quimera do Eclipse.".to_string(),
            condition: MilestoneCondition::SpecificSpecies(vec!["eclipse_chimera".to_string()]),
            reward: MilestoneReward {
                gold: 10000,
                food: 5000,
                speed_modifier: None,
            },
        });

        catalog
    }

    pub fn register(&mut self, milestone: CollectionMilestone) {
        self.milestones.insert(milestone.id.clone(), milestone);
    }

    pub fn get(&self, id: &str) -> Option<&CollectionMilestone> {
        self.milestones.get(id)
    }

    pub fn count(&self) -> usize {
        self.milestones.len()
    }

    /// Lista todos os marcos ordenados pelo id (ordem estável para menus de interface).
    pub fn all_sorted(&self) -> Vec<&CollectionMilestone> {
        let mut list: Vec<&CollectionMilestone> = self.milestones.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bestiary_discovery_and_completion() {
        let catalog = SpeciesCatalog::with_starter_species();
        let mut bestiary = Bestiary::new();

        assert_eq!(bestiary.discovered_count(), 0);
        assert_eq!(bestiary.completion_percentage(&catalog), 0.0);

        // Descoberta 1: Inédita
        assert!(bestiary.record_discovery("pyro_salamander", 1000));
        assert_eq!(bestiary.discovered_count(), 1);

        // Redescoberta da mesma espécie não é inédita
        assert!(!bestiary.record_discovery("pyro_salamander", 1010));
        assert_eq!(bestiary.discovered_count(), 1);

        // Atualização de nível
        bestiary.record_level("pyro_salamander", 5);
        assert_eq!(
            bestiary.discoveries.get("pyro_salamander").unwrap().highest_level_achieved,
            5
        );
    }

    #[test]
    fn test_milestone_claim_lifecycle() {
        let catalog = SpeciesCatalog::with_starter_species();
        let milestone_catalog = MilestoneCatalog::with_default_milestones();
        let mut bestiary = Bestiary::new();

        let mut gold = 100;
        let mut food = 50;

        // Tentar reivindicar "first_discovery" antes de qualquer descoberta deve falhar
        let res = bestiary.claim_milestone(
            "first_discovery",
            &milestone_catalog,
            &catalog,
            &mut gold,
            &mut food,
        );
        assert!(matches!(res, Err(BestiaryError::MilestoneConditionNotMet { .. })));

        // Registra descoberta
        bestiary.record_discovery("pyro_salamander", 1000);

        // Reivindica com sucesso
        let reward = bestiary
            .claim_milestone(
                "first_discovery",
                &milestone_catalog,
                &catalog,
                &mut gold,
                &mut food,
            )
            .expect("Should claim milestone successfully");

        assert_eq!(reward.gold, 200);
        assert_eq!(gold, 300); // 100 + 200
        assert_eq!(food, 150); // 50 + 100

        // Tentar reivindicar novamente deve falhar (anti-duplicação)
        let dup = bestiary.claim_milestone(
            "first_discovery",
            &milestone_catalog,
            &catalog,
            &mut gold,
            &mut food,
        );
        assert!(matches!(dup, Err(BestiaryError::MilestoneAlreadyClaimed { .. })));
    }
}
