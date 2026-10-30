//! Modelo de dados para espécies e instâncias de criaturas de Bestigard.
#![forbid(unsafe_code)]

use crate::element::{ElementId, ElementRegistry};
use serde::{Deserialize, Serialize};

/// Nível de raridade de uma espécie de criatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl Rarity {
    /// Fator de multiplicação de atributos gerais da raridade.
    pub fn stat_multiplier(&self) -> f32 {
        match self {
            Self::Common => 1.0,
            Self::Uncommon => 1.25,
            Self::Rare => 1.5,
            Self::Epic => 1.85,
            Self::Legendary => 2.3,
            Self::Mythic => 3.0,
        }
    }
}

/// Estágio de crescimento visual e funcional da criatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrowthStage {
    /// Nível 1 a 3: filhote fofo, menor geração de ouro e stats base.
    Baby,
    /// Nível 4 a 9: jovem em desenvolvimento, pronto para cruzamento.
    Juvenile,
    /// Nível 10+: adulto titânico, pico de poder de combate e geração econômica.
    Adult,
}

impl GrowthStage {
    /// Determina o estágio adequado com base no nível da criatura.
    pub fn from_level(level: u32) -> Self {
        match level {
            0..=3 => Self::Baby,
            4..=9 => Self::Juvenile,
            _ => Self::Adult,
        }
    }

    /// Multiplicador de escala de atributos de acordo com o estágio.
    pub fn stage_multiplier(&self) -> f32 {
        match self {
            Self::Baby => 0.8,
            Self::Juvenile => 1.0,
            Self::Adult => 1.35,
        }
    }
}

/// Definição estática de uma espécie de criatura (Template / Catálogo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureSpecies {
    pub id: String,
    pub name: String,
    pub description: String,
    pub elements: Vec<ElementId>,
    pub rarity: Rarity,
    pub base_hp: u32,
    pub base_attack: u32,
    pub base_defense: u32,
    pub base_speed: u32,
    pub base_gold_per_hour: u32,
    pub breeding_time_secs: u64,
    pub incubation_time_secs: u64,
}

impl CreatureSpecies {
    /// Valida se todos os elementos declarados para a espécie existem no registro.
    pub fn validate_elements(&self, registry: &ElementRegistry) -> bool {
        !self.elements.is_empty() && self.elements.iter().all(|el| registry.contains(el))
    }

    /// Retorna o preço em ouro para comprar o ovo desta espécie diretamente no mercado.
    /// Apenas espécies mono-elementais fundamentais podem ser compradas; híbridos exigem cruzamento (`None`).
    pub fn egg_shop_price_gold(&self) -> Option<u64> {
        if self.elements.len() != 1 {
            return None;
        }
        match self.rarity {
            Rarity::Common => Some(150),
            Rarity::Uncommon => Some(350),
            Rarity::Rare => Some(750),
            Rarity::Epic | Rarity::Legendary | Rarity::Mythic => None,
        }
    }
}

/// Instância única pertencente a um jogador no mundo de Bestigard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureInstance {
    pub instance_id: u64,
    pub species_id: String,
    pub nickname: Option<String>,
    pub level: u32,
    pub xp: u64,
    pub stage: GrowthStage,
    pub habitat_id: Option<u64>,
}

impl CreatureInstance {
    /// Cria uma nova criatura nível 1 no estágio Baby.
    pub fn new(instance_id: u64, species_id: impl Into<String>) -> Self {
        Self {
            instance_id,
            species_id: species_id.into(),
            nickname: None,
            level: 1,
            xp: 0,
            stage: GrowthStage::Baby,
            habitat_id: None,
        }
    }

    /// XP necessário para atingir o próximo nível a partir do nível atual.
    pub fn xp_for_next_level(&self) -> u64 {
        // Curva suave quadrática sem inflação artificial: 50 * level^1.8
        let lvl = self.level as f64;
        (50.0 * lvl.powf(1.8)).round() as u64
    }

    /// Adiciona experiência e processa múltiplos níveis e evoluções de estágio automaticamente.
    pub fn add_xp(&mut self, amount: u64) -> bool {
        self.xp += amount;
        let initial_level = self.level;

        while self.xp >= self.xp_for_next_level() {
            self.xp -= self.xp_for_next_level();
            self.level += 1;
        }

        self.stage = GrowthStage::from_level(self.level);
        self.level > initial_level
    }

    /// Calcula os pontos de vida (HP) máximos atuais com base no nível e estágio.
    pub fn calculate_hp(&self, species: &CreatureSpecies) -> u32 {
        let level_scaling = 1.0 + (self.level - 1) as f32 * 0.15;
        let val = species.base_hp as f32
            * species.rarity.stat_multiplier()
            * self.stage.stage_multiplier()
            * level_scaling;
        val.round() as u32
    }

    /// Calcula o poder de ataque atual.
    pub fn calculate_attack(&self, species: &CreatureSpecies) -> u32 {
        let level_scaling = 1.0 + (self.level - 1) as f32 * 0.15;
        let val = species.base_attack as f32
            * species.rarity.stat_multiplier()
            * self.stage.stage_multiplier()
            * level_scaling;
        val.round() as u32
    }

    /// Calcula a defesa atual.
    pub fn calculate_defense(&self, species: &CreatureSpecies) -> u32 {
        let level_scaling = 1.0 + (self.level - 1) as f32 * 0.12;
        let val = species.base_defense as f32
            * species.rarity.stat_multiplier()
            * self.stage.stage_multiplier()
            * level_scaling;
        val.round() as u32
    }

    /// Calcula a velocidade de iniciativa em combate.
    pub fn calculate_speed(&self, species: &CreatureSpecies) -> u32 {
        let level_scaling = 1.0 + (self.level - 1) as f32 * 0.05;
        let val = species.base_speed as f32 * level_scaling;
        val.round() as u32
    }

    /// Calcula a produção passiva de ouro por hora gerada pela criatura no habitat.
    pub fn calculate_gold_per_hour(&self, species: &CreatureSpecies) -> u32 {
        let level_scaling = 1.0 + (self.level - 1) as f32 * 0.25;
        let val = species.base_gold_per_hour as f32
            * species.rarity.stat_multiplier()
            * self.stage.stage_multiplier()
            * level_scaling;
        val.round() as u32
    }

    /// Retorna se a criatura tem idade suficiente para cruzar no ninho de reprodução.
    pub fn can_breed(&self) -> bool {
        self.stage != GrowthStage::Baby
    }
}

/// Catálogo e repositório de espécies disponíveis no jogo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpeciesCatalog {
    species: std::collections::HashMap<String, CreatureSpecies>,
}

impl SpeciesCatalog {
    pub fn new() -> Self {
        Self {
            species: std::collections::HashMap::new(),
        }
    }

    /// Cria um catálogo inicial contendo as criaturas fundamentais dos 7 elementos.
    pub fn with_starter_species() -> Self {
        let mut catalog = Self::new();

        catalog.register(CreatureSpecies {
            id: "pyro_salamander".to_string(),
            name: "Salamandra Ígnea".to_string(),
            description: "Réptil místico banhado em chamas eternas do vulcão primordial.".to_string(),
            elements: vec![ElementId::new("fire")],
            rarity: Rarity::Common,
            base_hp: 120,
            base_attack: 28,
            base_defense: 14,
            base_speed: 16,
            base_gold_per_hour: 40,
            breeding_time_secs: 30,
            incubation_time_secs: 30,
        });

        catalog.register(CreatureSpecies {
            id: "tide_serpent".to_string(),
            name: "Serpente das Marés".to_string(),
            description: "Criatura serpentina ágil que desliza por correntes e recifes abissais.".to_string(),
            elements: vec![ElementId::new("water")],
            rarity: Rarity::Common,
            base_hp: 140,
            base_attack: 22,
            base_defense: 18,
            base_speed: 18,
            base_gold_per_hour: 38,
            breeding_time_secs: 30,
            incubation_time_secs: 30,
        });

        catalog.register(CreatureSpecies {
            id: "gale_griffin".to_string(),
            name: "Grifo das Rajadas".to_string(),
            description: "Besta alada nobre dos picos gelados que cavalga o vento veloz.".to_string(),
            elements: vec![ElementId::new("air")],
            rarity: Rarity::Common,
            base_hp: 110,
            base_attack: 25,
            base_defense: 12,
            base_speed: 26,
            base_gold_per_hour: 42,
            breeding_time_secs: 30,
            incubation_time_secs: 30,
        });

        catalog.register(CreatureSpecies {
            id: "stone_golem".to_string(),
            name: "Golem Rochoso".to_string(),
            description: "Entidade monolítica ancestral esculpida a partir da pura crosta terrestre.".to_string(),
            elements: vec![ElementId::new("earth")],
            rarity: Rarity::Common,
            base_hp: 180,
            base_attack: 20,
            base_defense: 26,
            base_speed: 8,
            base_gold_per_hour: 35,
            breeding_time_secs: 30,
            incubation_time_secs: 30,
        });

        catalog.register(CreatureSpecies {
            id: "spark_falcon".to_string(),
            name: "Falcão Voltaico".to_string(),
            description: "Ave de rapina com penas condutoras que canalizam trovões celestes.".to_string(),
            elements: vec![ElementId::new("lightning")],
            rarity: Rarity::Uncommon,
            base_hp: 115,
            base_attack: 32,
            base_defense: 12,
            base_speed: 30,
            base_gold_per_hour: 55,
            breeding_time_secs: 60,
            incubation_time_secs: 60,
        });

        catalog.register(CreatureSpecies {
            id: "luminous_stag".to_string(),
            name: "Cervo da Alvorada".to_string(),
            description: "Espírito guardião radiante cujas galhadas emitem a luz do amanhecer.".to_string(),
            elements: vec![ElementId::new("light")],
            rarity: Rarity::Rare,
            base_hp: 150,
            base_attack: 30,
            base_defense: 22,
            base_speed: 22,
            base_gold_per_hour: 75,
            breeding_time_secs: 120,
            incubation_time_secs: 120,
        });

        catalog.register(CreatureSpecies {
            id: "void_stalker".to_string(),
            name: "Perseguidor Umbral".to_string(),
            description: "Predador enigmático que vagueia na fronteira entre a escuridão e a matéria.".to_string(),
            elements: vec![ElementId::new("shadow")],
            rarity: Rarity::Rare,
            base_hp: 135,
            base_attack: 36,
            base_defense: 16,
            base_speed: 25,
            base_gold_per_hour: 80,
            breeding_time_secs: 120,
            incubation_time_secs: 120,
        });

        catalog.register(CreatureSpecies {
            id: "volcano_beast".to_string(),
            name: "Besta Vulcânica".to_string(),
            description: "Colosso nascido da união do fogo abrasador e da rocha tectônica.".to_string(),
            elements: vec![ElementId::new("fire"), ElementId::new("earth")],
            rarity: Rarity::Uncommon,
            base_hp: 210,
            base_attack: 34,
            base_defense: 25,
            base_speed: 12,
            base_gold_per_hour: 60,
            breeding_time_secs: 45,
            incubation_time_secs: 45,
        });

        catalog.register(CreatureSpecies {
            id: "steam_naga".to_string(),
            name: "Naga de Vapor".to_string(),
            description: "Espírito hidro-térmico capaz de expelir névoas ferventes de camuflagem.".to_string(),
            elements: vec![ElementId::new("fire"), ElementId::new("water")],
            rarity: Rarity::Uncommon,
            base_hp: 160,
            base_attack: 30,
            base_defense: 18,
            base_speed: 22,
            base_gold_per_hour: 62,
            breeding_time_secs: 45,
            incubation_time_secs: 45,
        });

        catalog.register(CreatureSpecies {
            id: "storm_roc".to_string(),
            name: "Roc da Tempestade".to_string(),
            description: "Pássaro lendário que cruza vendavais elétricos anunciando tormentas.".to_string(),
            elements: vec![ElementId::new("air"), ElementId::new("lightning")],
            rarity: Rarity::Rare,
            base_hp: 170,
            base_attack: 38,
            base_defense: 18,
            base_speed: 32,
            base_gold_per_hour: 90,
            breeding_time_secs: 90,
            incubation_time_secs: 90,
        });

        catalog.register(CreatureSpecies {
            id: "eclipse_chimera".to_string(),
            name: "Quimera do Eclipse".to_string(),
            description: "Entidade mítica rara gerada no alinhamento singular entre pura luz e sombras abissais.".to_string(),
            elements: vec![ElementId::new("light"), ElementId::new("shadow")],
            rarity: Rarity::Epic,
            base_hp: 250,
            base_attack: 48,
            base_defense: 30,
            base_speed: 28,
            base_gold_per_hour: 150,
            breeding_time_secs: 180,
            incubation_time_secs: 180,
        });

        catalog
    }


    pub fn register(&mut self, species: CreatureSpecies) {
        self.species.insert(species.id.clone(), species);
    }

    pub fn get(&self, id: &str) -> Option<&CreatureSpecies> {
        self.species.get(id)
    }

    pub fn all(&self) -> impl Iterator<Item = &CreatureSpecies> {
        self.species.values()
    }

    pub fn count(&self) -> usize {
        self.species.len()
    }

    /// Lista todas as espécies disponíveis para compra direta de ovo no mercado, ordenadas por preço e ID.
    pub fn shop_eggs_sorted(&self) -> Vec<(&CreatureSpecies, u64)> {
        let mut list: Vec<(&CreatureSpecies, u64)> = self
            .species
            .values()
            .filter_map(|s| s.egg_shop_price_gold().map(|price| (s, price)))
            .collect();
        list.sort_by(|(a, pa), (b, pb)| pa.cmp(pb).then_with(|| a.id.cmp(&b.id)));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starter_species_validation() {
        let registry = ElementRegistry::with_default_elements();
        let catalog = SpeciesCatalog::with_starter_species();

        assert_eq!(catalog.count(), 11);

        for species in catalog.species.values() {
            assert!(
                species.validate_elements(&registry),
                "Espécie {} possui elemento inválido!",
                species.name
            );
        }
    }

    #[test]
    fn test_creature_leveling_and_evolution() {
        let catalog = SpeciesCatalog::with_starter_species();
        let pyro = catalog.get("pyro_salamander").expect("Species should exist");

        let mut creature = CreatureInstance::new(1, &pyro.id);
        assert_eq!(creature.level, 1);
        assert_eq!(creature.stage, GrowthStage::Baby);
        assert!(!creature.can_breed());

        let hp_lvl1 = creature.calculate_hp(pyro);
        let gold_lvl1 = creature.calculate_gold_per_hour(pyro);

        // Dá XP suficiente para chegar ao nível 5 (Juvenile)
        creature.add_xp(1000);
        assert!(creature.level >= 4);
        assert_eq!(creature.stage, GrowthStage::Juvenile);
        assert!(creature.can_breed());

        let hp_lvl5 = creature.calculate_hp(pyro);
        let gold_lvl5 = creature.calculate_gold_per_hour(pyro);

        assert!(hp_lvl5 > hp_lvl1);
        assert!(gold_lvl5 > gold_lvl1);

        // Dá mais XP para atingir nível 10+ (Adult)
        creature.add_xp(10000);
        assert!(creature.level >= 10);
        assert_eq!(creature.stage, GrowthStage::Adult);
    }
}
