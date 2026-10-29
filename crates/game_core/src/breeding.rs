//! Motor determinístico de cruzamento e genética de criaturas.
#![forbid(unsafe_code)]

use crate::creature::{CreatureInstance, CreatureSpecies, Rarity, SpeciesCatalog};
use crate::element::ElementId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Erros possíveis durante a tentativa de cruzamento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreedingError {
    ParentCannotBreed { instance_id: u64, reason: String },
    SameInstance,
    NoEligibleOffspring,
}

impl std::fmt::Display for BreedingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParentCannotBreed { instance_id, reason } => {
                write!(f, "Criatura #{} não pode cruzar: {}", instance_id, reason)
            }
            Self::SameInstance => write!(f, "Uma criatura não pode cruzar consigo mesma"),
            Self::NoEligibleOffspring => write!(f, "Nenhuma espécie compatível encontrada para os elementos dos pais"),
        }
    }
}

impl std::error::Error for BreedingError {}

/// Resultado gerado após o início do processo de cruzamento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreedingResult {
    pub offspring_species_id: String,
    pub duration_secs: u64,
    pub parent_a_id: u64,
    pub parent_b_id: u64,
}

/// Gerador de números pseudo-aleatórios determinístico (Xorshift64*).
/// Garante que o mesmo seed produza o mesmo resultado em qualquer plataforma.
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        // Evita estado zero inválido no Xorshift
        let state = if seed == 0 { 0x853c49e6748fea9b } else { seed };
        Self { state }
    }

    /// Retorna um inteiro de 64 bits pseudo-aleatório.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545f4914f6cdd1d)
    }

    /// Retorna um valor no intervalo [0.0, 1.0).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Escolhe um índice com base em uma lista de pesos acumulados.
    pub fn choose_weighted(&mut self, weights: &[f32]) -> Option<usize> {
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            return None;
        }

        let roll = self.next_f32() * total;
        let mut accumulator = 0.0;
        for (i, &w) in weights.iter().enumerate() {
            accumulator += w;
            if roll <= accumulator {
                return Some(i);
            }
        }

        Some(weights.len() - 1)
    }
}

/// Motor de regras de cruzamento genético de Bestigard.
#[derive(Debug, Clone, Default)]
pub struct BreedingEngine;

impl BreedingEngine {
    pub fn new() -> Self {
        Self
    }

    /// Reúne todos os elementos únicos contidos no conjunto dos pais.
    pub fn extract_parent_elements(
        &self,
        species_a: &CreatureSpecies,
        species_b: &CreatureSpecies,
    ) -> HashSet<ElementId> {
        let mut elements = HashSet::new();
        for el in &species_a.elements {
            elements.insert(el.clone());
        }
        for el in &species_b.elements {
            elements.insert(el.clone());
        }
        elements
    }

    /// Encontra no catálogo todas as espécies cujos elementos são um subconjunto
    /// dos elementos combinados dos progenitores.
    pub fn find_candidate_species<'a>(
        &self,
        species_a: &CreatureSpecies,
        species_b: &CreatureSpecies,
        catalog: &'a SpeciesCatalog,
    ) -> Vec<&'a CreatureSpecies> {
        let available_elements = self.extract_parent_elements(species_a, species_b);

        catalog
            .all()
            .filter(|species| {
                // A espécie filha só é elegível se todos os seus elementos vierem dos pais
                !species.elements.is_empty()
                    && species
                        .elements
                        .iter()
                        .all(|el| available_elements.contains(el))
            })
            .collect()
    }

    /// Calcula o peso de probabilidade de nascimento de uma espécie candidata.
    /// Considera a raridade da espécie e concede bônus com base no nível médio dos pais.
    pub fn calculate_offspring_weight(
        &self,
        candidate: &CreatureSpecies,
        avg_parent_level: f32,
    ) -> f32 {
        let base_weight = match candidate.rarity {
            Rarity::Common => 100.0,
            Rarity::Uncommon => 50.0,
            Rarity::Rare => 22.0,
            Rarity::Epic => 8.0,
            Rarity::Legendary => 2.0,
            Rarity::Mythic => 0.5,
        };

        // Bônus de sorte por dedicação do jogador: pais de nível alto aumentam a chance de espécies raras
        let rarity_tier = match candidate.rarity {
            Rarity::Common => 0.0,
            Rarity::Uncommon => 1.0,
            Rarity::Rare => 2.0,
            Rarity::Epic => 3.0,
            Rarity::Legendary => 4.0,
            Rarity::Mythic => 5.0,
        };

        // Bônus de 2% por nível dos pais para raridades maiores
        let level_bonus = 1.0 + (avg_parent_level.max(1.0) - 1.0) * 0.02 * rarity_tier;

        base_weight * level_bonus
    }

    /// Executa o cruzamento determinístico entre duas criaturas.
    pub fn breed(
        &self,
        parent_a: &CreatureInstance,
        species_a: &CreatureSpecies,
        parent_b: &CreatureInstance,
        species_b: &CreatureSpecies,
        catalog: &SpeciesCatalog,
        seed: u64,
    ) -> Result<BreedingResult, BreedingError> {
        // 1. Validações básicas de maturidade e individualidade
        if parent_a.instance_id == parent_b.instance_id {
            return Err(BreedingError::SameInstance);
        }

        if !parent_a.can_breed() {
            return Err(BreedingError::ParentCannotBreed {
                instance_id: parent_a.instance_id,
                reason: "A criatura ainda é um filhote (Baby). Suba-a para o nível 4+".to_string(),
            });
        }

        if !parent_b.can_breed() {
            return Err(BreedingError::ParentCannotBreed {
                instance_id: parent_b.instance_id,
                reason: "A criatura ainda é um filhote (Baby). Suba-a para o nível 4+".to_string(),
            });
        }

        // 2. Busca espécies filhas elegíveis
        let candidates = self.find_candidate_species(species_a, species_b, catalog);
        if candidates.is_empty() {
            return Err(BreedingError::NoEligibleOffspring);
        }

        // 3. Monta roleta de probabilidade ponderada
        let avg_level = (parent_a.level + parent_b.level) as f32 / 2.0;
        let weights: Vec<f32> = candidates
            .iter()
            .map(|c| self.calculate_offspring_weight(c, avg_level))
            .collect();

        let mut rng = DeterministicRng::new(seed);
        let chosen_idx = rng
            .choose_weighted(&weights)
            .unwrap_or(0);

        let chosen_species = candidates[chosen_idx];

        // 4. Calcula duração do cruzamento com base no catálogo (equilibrado sem timers abusivos)
        let duration = chosen_species.breeding_time_secs;

        Ok(BreedingResult {
            offspring_species_id: chosen_species.id.clone(),
            duration_secs: duration,
            parent_a_id: parent_a.instance_id,
            parent_b_id: parent_b.instance_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::ElementId;

    #[test]
    fn test_rng_determinism() {
        let mut rng1 = DeterministicRng::new(42);
        let mut rng2 = DeterministicRng::new(42);

        for _ in 0..10 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_baby_cannot_breed() {
        let catalog = SpeciesCatalog::with_starter_species();
        let engine = BreedingEngine::new();

        let parent_a = CreatureInstance::new(1, "pyro_salamander"); // Nível 1 = Baby
        let mut parent_b = CreatureInstance::new(2, "stone_golem");
        parent_b.add_xp(1000); // Nível 4+ = Juvenile

        let species_a = catalog.get(&parent_a.species_id).unwrap();
        let species_b = catalog.get(&parent_b.species_id).unwrap();

        let result = engine.breed(&parent_a, species_a, &parent_b, species_b, &catalog, 123);
        assert!(matches!(result, Err(BreedingError::ParentCannotBreed { .. })));
    }

    #[test]
    fn test_successful_deterministic_breeding() {
        let mut catalog = SpeciesCatalog::with_starter_species();

        // Adiciona um híbrido de Fogo + Terra
        catalog.register(CreatureSpecies {
            id: "volcano_beast".to_string(),
            name: "Besta Vulcânica".to_string(),
            description: "Fusão de fogo e magma rochoso.".to_string(),
            elements: vec![ElementId::new("fire"), ElementId::new("earth")],
            rarity: Rarity::Uncommon,
            base_hp: 200,
            base_attack: 34,
            base_defense: 25,
            base_speed: 12,
            base_gold_per_hour: 60,
            breeding_time_secs: 45,
            incubation_time_secs: 45,
        });

        let engine = BreedingEngine::new();

        let mut parent_a = CreatureInstance::new(1, "pyro_salamander");
        parent_a.add_xp(1000); // Juvenile
        let mut parent_b = CreatureInstance::new(2, "stone_golem");
        parent_b.add_xp(1000); // Juvenile

        let species_a = catalog.get(&parent_a.species_id).unwrap();
        let species_b = catalog.get(&parent_b.species_id).unwrap();

        // Cruzamento com a mesma seed DEVE ser 100% reproduzível
        let res1 = engine.breed(&parent_a, species_a, &parent_b, species_b, &catalog, 9999).unwrap();
        let res2 = engine.breed(&parent_a, species_a, &parent_b, species_b, &catalog, 9999).unwrap();

        assert_eq!(res1, res2);
        assert!(
            res1.offspring_species_id == "pyro_salamander"
                || res1.offspring_species_id == "stone_golem"
                || res1.offspring_species_id == "volcano_beast"
        );
    }
}
