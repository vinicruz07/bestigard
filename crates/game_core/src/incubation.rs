//! Sistema de incubação de ovos, ninho (hatchery) e aceleradores genéricos.
#![forbid(unsafe_code)]

use crate::creature::{CreatureInstance, CreatureSpecies, GrowthStage, SpeciesCatalog};
use serde::{Deserialize, Serialize};

/// Erros relacionados à incubação e eclosão de ovos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HatcheryError {
    HatcheryFull { max_slots: usize },
    EggNotFound { egg_id: u64 },
    EggNotReady { remaining_secs: u64 },
    SpeciesNotFound { species_id: String },
}

impl std::fmt::Display for HatcheryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HatcheryFull { max_slots } => {
                write!(f, "O ninho está lotado (capacidade máxima de {} ovos)", max_slots)
            }
            Self::EggNotFound { egg_id } => {
                write!(f, "Ovo #{} não encontrado no ninho", egg_id)
            }
            Self::EggNotReady { remaining_secs } => {
                write!(f, "Ovo ainda não está pronto para chocar (restam {}s)", remaining_secs)
            }
            Self::SpeciesNotFound { species_id } => {
                write!(f, "Espécie '{}' não encontrada no catálogo", species_id)
            }
        }
    }
}

impl std::error::Error for HatcheryError {}

/// Origem genérica de um acelerador de incubação (item, amuleto, bênção, habilidade passiva).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierSource {
    Item(String),
    Amulet(String),
    PassiveSkill(String),
    SanctuaryBlessing(String),
}

/// Modificador genérico que acelera a velocidade ou reduz o tempo de choca de ovos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncubationSpeedModifier {
    pub id: String,
    pub name: String,
    pub source: ModifierSource,
    /// Multiplicador de velocidade (ex: 1.5 = +50% de velocidade de avanço temporal, 2.0 = dobro).
    pub speed_multiplier: f32,
    /// Duração restante do modificador em segundos (None = efeito permanente/passivo).
    pub duration_remaining_secs: Option<f64>,
}

impl IncubationSpeedModifier {
    /// Cria um modificador percentual de velocidade permanente (ex: amuleto equipado ou passiva de guardião).
    pub fn permanent(
        id: impl Into<String>,
        name: impl Into<String>,
        source: ModifierSource,
        speed_multiplier: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            source,
            speed_multiplier: speed_multiplier.max(1.0),
            duration_remaining_secs: None,
        }
    }

    /// Cria um modificador temporário (ex: incenso, poção solar ou item consumível de tempo).
    pub fn temporary(
        id: impl Into<String>,
        name: impl Into<String>,
        source: ModifierSource,
        speed_multiplier: f32,
        duration_secs: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            source,
            speed_multiplier: speed_multiplier.max(1.0),
            duration_remaining_secs: Some(duration_secs.max(0.0)),
        }
    }
}

/// Instância de um ovo em desenvolvimento no ninho.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EggInstance {
    pub egg_id: u64,
    pub species_id: String,
    pub elapsed_secs: f64,
    pub total_duration_secs: u64,
}

impl EggInstance {
    pub fn new(egg_id: u64, species: &CreatureSpecies) -> Self {
        Self {
            egg_id,
            species_id: species.id.clone(),
            elapsed_secs: 0.0,
            total_duration_secs: species.incubation_time_secs,
        }
    }

    /// Retorna se o ovo já está pronto para eclosão.
    pub fn is_ready(&self) -> bool {
        self.elapsed_secs >= self.total_duration_secs as f64
    }

    /// Segundos restantes para a eclosão.
    pub fn remaining_secs(&self) -> u64 {
        if self.is_ready() {
            0
        } else {
            (self.total_duration_secs as f64 - self.elapsed_secs).ceil() as u64
        }
    }

    /// Reduz instantaneamente o tempo restante (ex: catalisador arcano ou sopro elemental).
    pub fn apply_instant_reduction(&mut self, secs: u64) {
        self.elapsed_secs = (self.elapsed_secs + secs as f64).min(self.total_duration_secs as f64);
    }
}

/// Ninho / Incubadora de ovos na ilha de Bestigard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HatcheryInstance {
    pub instance_id: u64,
    pub max_slots: usize,
    pub eggs: Vec<EggInstance>,
    pub modifiers: Vec<IncubationSpeedModifier>,
    next_egg_id_counter: u64,
}

impl HatcheryInstance {
    pub fn new(instance_id: u64, max_slots: usize) -> Self {
        Self {
            instance_id,
            max_slots: max_slots.max(1),
            eggs: Vec::new(),
            modifiers: Vec::new(),
            next_egg_id_counter: 1,
        }
    }

    /// Retorna se há pelo menos um slot livre no ninho para novo ovo.
    pub fn can_add_egg(&self) -> bool {
        self.eggs.len() < self.max_slots
    }

    /// Adiciona um novo ovo ao ninho caso haja slot disponível.
    pub fn add_egg(&mut self, species: &CreatureSpecies) -> Result<u64, HatcheryError> {
        if self.eggs.len() >= self.max_slots {
            return Err(HatcheryError::HatcheryFull {
                max_slots: self.max_slots,
            });
        }

        let egg_id = self.next_egg_id_counter;
        self.next_egg_id_counter += 1;

        let egg = EggInstance::new(egg_id, species);
        self.eggs.push(egg);

        Ok(egg_id)
    }

    /// Adiciona ou substitui um acelerador genérico de incubação.
    pub fn add_modifier(&mut self, modifier: IncubationSpeedModifier) {
        self.modifiers.retain(|m| m.id != modifier.id);
        self.modifiers.push(modifier);
    }

    /// Remove um acelerador pelo ID.
    pub fn remove_modifier(&mut self, modifier_id: &str) -> bool {
        let initial_len = self.modifiers.len();
        self.modifiers.retain(|m| m.id != modifier_id);
        self.modifiers.len() < initial_len
    }

    /// Calcula o multiplicador total de velocidade de incubação (efeitos multiplicativos e cumulativos).
    pub fn total_speed_multiplier(&self) -> f32 {
        let mut total = 1.0f32;
        for m in &self.modifiers {
            total *= m.speed_multiplier;
        }
        total
    }

    /// Atualiza o avanço da incubação de todos os ovos, aplicando os bônus de velocidade.
    pub fn update_incubation(&mut self, real_dt_secs: f64) {
        if real_dt_secs <= 0.0 {
            return;
        }

        let speed = self.total_speed_multiplier() as f64;
        let accelerated_dt = real_dt_secs * speed;

        // Avança o tempo de todos os ovos
        for egg in &mut self.eggs {
            if !egg.is_ready() {
                egg.elapsed_secs = (egg.elapsed_secs + accelerated_dt).min(egg.total_duration_secs as f64);
            }
        }

        // Atualiza a duração dos modificadores temporários e expira os esgotados
        for m in &mut self.modifiers {
            if let Some(remaining) = &mut m.duration_remaining_secs {
                *remaining -= real_dt_secs;
            }
        }
        self.modifiers.retain(|m| {
            m.duration_remaining_secs
                .map(|rem| rem > 0.0)
                .unwrap_or(true)
        });
    }

    /// Choca um ovo pronto, convertendo-o em uma nova `CreatureInstance` filhote (Baby Nível 1).
    pub fn hatch_egg(
        &mut self,
        egg_id: u64,
        new_creature_instance_id: u64,
        catalog: &SpeciesCatalog,
    ) -> Result<CreatureInstance, HatcheryError> {
        let idx = self
            .eggs
            .iter()
            .position(|e| e.egg_id == egg_id)
            .ok_or(HatcheryError::EggNotFound { egg_id })?;

        let egg = &self.eggs[idx];

        if !egg.is_ready() {
            return Err(HatcheryError::EggNotReady {
                remaining_secs: egg.remaining_secs(),
            });
        }

        let species_id = egg.species_id.clone();
        if catalog.get(&species_id).is_none() {
            return Err(HatcheryError::SpeciesNotFound { species_id });
        }

        // Remove o ovo do ninho
        self.eggs.remove(idx);

        // Cria a criatura recém-nascida nível 1
        let mut creature = CreatureInstance::new(new_creature_instance_id, species_id);
        creature.stage = GrowthStage::Baby;

        Ok(creature)
    }

    /// Libera ou vende um ovo que o jogador não queira chocar, concedendo ouro equivalente à sua raridade.
    pub fn sell_or_release_egg(
        &mut self,
        egg_id: u64,
        catalog: &SpeciesCatalog,
    ) -> Result<u64, HatcheryError> {
        let idx = self
            .eggs
            .iter()
            .position(|e| e.egg_id == egg_id)
            .ok_or(HatcheryError::EggNotFound { egg_id })?;

        let egg = self.eggs.remove(idx);
        let species = catalog
            .get(&egg.species_id)
            .ok_or(HatcheryError::SpeciesNotFound {
                species_id: egg.species_id,
            })?;

        // Ouro justo de compensação pela venda do ovo
        let gold_reward = (species.base_gold_per_hour as f32 * species.rarity.stat_multiplier() * 2.0).round() as u64;

        Ok(gold_reward)
    }

    /// Quantidade de ovos atualmente no ninho.
    pub fn egg_count(&self) -> usize {
        self.eggs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hatchery_lifecycle_and_capacity() {
        let catalog = SpeciesCatalog::with_starter_species();
        let pyro = catalog.get("pyro_salamander").unwrap();
        let water = catalog.get("tide_serpent").unwrap();
        let earth = catalog.get("stone_golem").unwrap();

        let mut hatchery = HatcheryInstance::new(1, 2); // 2 slots máximos

        let egg1 = hatchery.add_egg(pyro).expect("Should add egg 1");
        let _egg2 = hatchery.add_egg(water).expect("Should add egg 2");

        // 3º ovo em ninho de 2 slots deve falhar
        assert!(matches!(
            hatchery.add_egg(earth),
            Err(HatcheryError::HatcheryFull { .. })
        ));

        // Tentar chocar antes do tempo falha
        assert!(matches!(
            hatchery.hatch_egg(egg1, 100, &catalog),
            Err(HatcheryError::EggNotReady { .. })
        ));

        // Avança tempo base
        hatchery.update_incubation(pyro.incubation_time_secs as f64);

        // Choca com sucesso
        let creature = hatchery.hatch_egg(egg1, 100, &catalog).expect("Should hatch");
        assert_eq!(creature.instance_id, 100);
        assert_eq!(creature.species_id, "pyro_salamander");
        assert_eq!(creature.stage, GrowthStage::Baby);
        assert_eq!(hatchery.egg_count(), 1); // 1 ovo restante no ninho
    }

    #[test]
    fn test_incubation_speed_modifiers() {
        let catalog = SpeciesCatalog::with_starter_species();
        let stag = catalog.get("luminous_stag").unwrap(); // 120s de incubação

        let mut hatchery = HatcheryInstance::new(1, 4);
        let egg_id = hatchery.add_egg(stag).unwrap();

        // Adiciona um Amuleto de Aceleração (2x de velocidade permanente)
        hatchery.add_modifier(IncubationSpeedModifier::permanent(
            "dawn_amulet",
            "Amuleto da Alvorada",
            ModifierSource::Amulet("dawn_amulet".to_string()),
            2.0,
        ));

        // Adiciona um Incenso Temporal (mais 1.5x de velocidade por 60s)
        hatchery.add_modifier(IncubationSpeedModifier::temporary(
            "solar_incense",
            "Incenso Solar",
            ModifierSource::Item("solar_incense".to_string()),
            1.5,
            60.0,
        ));

        // Multiplicador combinado: 2.0 * 1.5 = 3.0x de velocidade!
        assert!((hatchery.total_speed_multiplier() - 3.0).abs() < 1e-4);

        // Avança 20 segundos reais -> com 3x equivale a 60 segundos de incubação!
        hatchery.update_incubation(20.0);

        let egg = &hatchery.eggs[0];
        assert_eq!(egg.egg_id, egg_id);
        assert!((egg.elapsed_secs - 60.0).abs() < 1e-3);
        assert_eq!(egg.remaining_secs(), 60);

        // Mais 20 segundos reais -> 60s adicionais de progresso (total 120s)
        hatchery.update_incubation(20.0);
        assert!(hatchery.eggs[0].is_ready());
    }
}
