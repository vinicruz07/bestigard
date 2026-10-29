//! Sistema de habitats, grid de ilhas e geração passiva de recursos.
#![forbid(unsafe_code)]

use crate::creature::{CreatureInstance, SpeciesCatalog};
use crate::element::ElementId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Erros relacionados ao gerenciamento de habitats e grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HabitatError {
    ElementNotSupported {
        creature_id: u64,
        element: ElementId,
    },
    HabitatFull {
        habitat_id: u64,
        capacity: usize,
    },
    PositionOutOfBounds {
        x: u32,
        y: u32,
    },
    TileOccupied {
        x: u32,
        y: u32,
    },
    HabitatNotFound {
        habitat_id: u64,
    },
    MaxTierReached,
}

impl std::fmt::Display for HabitatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ElementNotSupported { creature_id, element } => {
                write!(f, "Criatura #{} não possui afinidade com o elemento '{}' deste habitat", creature_id, element)
            }
            Self::HabitatFull { habitat_id, capacity } => {
                write!(f, "Habitat #{} atingiu a capacidade máxima de {} criaturas", habitat_id, capacity)
            }
            Self::PositionOutOfBounds { x, y } => {
                write!(f, "Posição ({}, {}) está fora dos limites da ilha", x, y)
            }
            Self::TileOccupied { x, y } => {
                write!(f, "A célula ({}, {}) já está ocupada por outra estrutura", x, y)
            }
            Self::HabitatNotFound { habitat_id } => {
                write!(f, "Habitat #{} não foi encontrado", habitat_id)
            }
            Self::MaxTierReached => write!(f, "O habitat já está no nível máximo"),
        }
    }
}

impl std::error::Error for HabitatError {}

/// Informações de evolução e capacidade de um nível (Tier) do habitat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatTierInfo {
    pub tier: u32,
    pub max_creatures: usize,
    pub gold_capacity: u64,
    pub upgrade_cost_gold: u64,
}

/// Especificação estática do tipo de habitat no catálogo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitatType {
    pub id: String,
    pub name: String,
    pub supported_elements: Vec<ElementId>,
    pub width: u32,
    pub height: u32,
    pub tiers: Vec<HabitatTierInfo>,
}

impl HabitatType {
    /// Retorna as informações do tier solicitado (1-based).
    pub fn get_tier_info(&self, tier: u32) -> Option<&HabitatTierInfo> {
        self.tiers.iter().find(|t| t.tier == tier)
    }

    /// Verifica se o habitat aceita uma criatura com base nos seus elementos.
    pub fn accepts_elements(&self, elements: &[ElementId]) -> bool {
        elements.iter().any(|el| self.supported_elements.contains(el))
    }
}

/// Instância de habitat construída na ilha do jogador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatInstance {
    pub instance_id: u64,
    pub type_id: String,
    pub grid_x: u32,
    pub grid_y: u32,
    pub tier: u32,
    pub creature_ids: Vec<u64>,
    pub accumulated_gold: u64,
}

impl HabitatInstance {
    pub fn new(instance_id: u64, type_id: impl Into<String>, grid_x: u32, grid_y: u32) -> Self {
        Self {
            instance_id,
            type_id: type_id.into(),
            grid_x,
            grid_y,
            tier: 1,
            creature_ids: Vec::new(),
            accumulated_gold: 0,
        }
    }

    /// Adiciona uma criatura ao habitat respeitando capacidade e elementos.
    pub fn add_creature(
        &mut self,
        creature: &CreatureInstance,
        hab_type: &HabitatType,
        catalog: &SpeciesCatalog,
    ) -> Result<(), HabitatError> {
        let tier_info = hab_type.get_tier_info(self.tier).ok_or(HabitatError::MaxTierReached)?;

        if self.creature_ids.len() >= tier_info.max_creatures {
            return Err(HabitatError::HabitatFull {
                habitat_id: self.instance_id,
                capacity: tier_info.max_creatures,
            });
        }

        let species = catalog.get(&creature.species_id).ok_or_else(|| {
            HabitatError::ElementNotSupported {
                creature_id: creature.instance_id,
                element: ElementId::new("unknown"),
            }
        })?;

        if !hab_type.accepts_elements(&species.elements) {
            let primary_element = species.elements.first().cloned().unwrap_or_else(|| ElementId::new("none"));
            return Err(HabitatError::ElementNotSupported {
                creature_id: creature.instance_id,
                element: primary_element,
            });
        }

        if !self.creature_ids.contains(&creature.instance_id) {
            self.creature_ids.push(creature.instance_id);
        }

        Ok(())
    }

    /// Remove uma criatura do habitat.
    pub fn remove_creature(&mut self, creature_id: u64) -> bool {
        if let Some(pos) = self.creature_ids.iter().position(|&id| id == creature_id) {
            self.creature_ids.remove(pos);
            true
        } else {
            false
        }
    }

    /// Atualiza a geração de ouro passivo a partir dos segundos decorridos.
    /// Respeita estritamente o teto máximo de capacidade de ouro do tier atual.
    pub fn generate_gold(
        &mut self,
        elapsed_seconds: f64,
        creatures: &HashMap<u64, CreatureInstance>,
        catalog: &SpeciesCatalog,
        hab_type: &HabitatType,
    ) {
        if self.creature_ids.is_empty() || elapsed_seconds <= 0.0 {
            return;
        }

        let tier_info = match hab_type.get_tier_info(self.tier) {
            Some(info) => info,
            None => return,
        };

        // Calcula taxa total de ouro por hora das criaturas abrigadas
        let mut total_gold_per_hour = 0u64;
        for &cid in &self.creature_ids {
            if let Some(inst) = creatures.get(&cid) {
                if let Some(species) = catalog.get(&inst.species_id) {
                    total_gold_per_hour += inst.calculate_gold_per_hour(species) as u64;
                }
            }
        }

        // Converte ouro por hora para a fração de segundos decorrida
        let generated = (total_gold_per_hour as f64 * (elapsed_seconds / 3600.0)).round() as u64;

        self.accumulated_gold = (self.accumulated_gold + generated).min(tier_info.gold_capacity);
    }

    /// Coleta todo o ouro acumulado no habitat e zera o cofre.
    pub fn collect_gold(&mut self) -> u64 {
        let gold = self.accumulated_gold;
        self.accumulated_gold = 0;
        gold
    }

    /// Realiza o upgrade do habitat para o próximo nível.
    pub fn upgrade(&mut self, hab_type: &HabitatType) -> Result<u64, HabitatError> {
        let next_tier = self.tier + 1;
        let next_info = hab_type.get_tier_info(next_tier).ok_or(HabitatError::MaxTierReached)?;
        self.tier = next_tier;
        Ok(next_info.upgrade_cost_gold)
    }
}

/// Grid isométrico 2D representando o terreno de uma ilha de Bestigard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandGrid {
    pub width: u32,
    pub height: u32,
    /// Mapa de células ocupadas ("x:y" -> ID da instância do habitat)
    occupied_cells: HashMap<String, u64>,
}

impl IslandGrid {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            occupied_cells: HashMap::new(),
        }
    }

    #[inline]
    pub fn cell_key(x: u32, y: u32) -> String {
        format!("{}:{}", x, y)
    }

    /// Verifica se uma área retangular está dentro dos limites da ilha e totalmente livre.
    pub fn can_place(
        &self,
        origin_x: u32,
        origin_y: u32,
        w: u32,
        h: u32,
        ignore_habitat_id: Option<u64>,
    ) -> Result<(), HabitatError> {
        if origin_x + w > self.width || origin_y + h > self.height {
            return Err(HabitatError::PositionOutOfBounds {
                x: origin_x + w,
                y: origin_y + h,
            });
        }

        for x in origin_x..origin_x + w {
            for y in origin_y..origin_y + h {
                if let Some(&occupier) = self.occupied_cells.get(&Self::cell_key(x, y)) {
                    if Some(occupier) != ignore_habitat_id {
                        return Err(HabitatError::TileOccupied { x, y });
                    }
                }
            }
        }

        Ok(())
    }

    /// Registra a ocupação das células do grid para um habitat.
    pub fn place_habitat(
        &mut self,
        habitat: &HabitatInstance,
        hab_type: &HabitatType,
    ) -> Result<(), HabitatError> {
        self.can_place(habitat.grid_x, habitat.grid_y, hab_type.width, hab_type.height, None)?;

        for x in habitat.grid_x..habitat.grid_x + hab_type.width {
            for y in habitat.grid_y..habitat.grid_y + hab_type.height {
                self.occupied_cells.insert(Self::cell_key(x, y), habitat.instance_id);
            }
        }

        Ok(())
    }

    /// Remove a ocupação das células de um habitat.
    pub fn remove_habitat(&mut self, habitat: &HabitatInstance, hab_type: &HabitatType) {
        for x in habitat.grid_x..habitat.grid_x + hab_type.width {
            for y in habitat.grid_y..habitat.grid_y + hab_type.height {
                let key = Self::cell_key(x, y);
                if self.occupied_cells.get(&key) == Some(&habitat.instance_id) {
                    self.occupied_cells.remove(&key);
                }
            }
        }
    }

    /// Move um habitat para uma nova posição no grid.
    pub fn move_habitat(
        &mut self,
        habitat: &mut HabitatInstance,
        hab_type: &HabitatType,
        new_x: u32,
        new_y: u32,
    ) -> Result<(), HabitatError> {
        // Valida se o novo destino está livre, ignorando o próprio habitat
        self.can_place(new_x, new_y, hab_type.width, hab_type.height, Some(habitat.instance_id))?;

        // Desocupa as posições antigas
        self.remove_habitat(habitat, hab_type);

        // Atualiza coordenadas e ocupa novas células
        habitat.grid_x = new_x;
        habitat.grid_y = new_y;
        self.place_habitat(habitat, hab_type)
    }

    /// Retorna se uma célula específica está ocupada.
    pub fn is_occupied(&self, x: u32, y: u32) -> bool {
        self.occupied_cells.contains_key(&Self::cell_key(x, y))
    }
}


/// Catálogo padrão de tipos de habitat para os elementos de Bestigard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HabitatCatalog {
    types: HashMap<String, HabitatType>,
}

impl HabitatCatalog {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    /// Retorna catálogo com os habitats elementais padrão (3x3 tiles, 3 tiers).
    pub fn with_default_habitats() -> Self {
        let mut catalog = Self::new();

        let elements = [
            ("fire_habitat", "Habitat de Fogo", "fire"),
            ("water_habitat", "Habitat de Água", "water"),
            ("air_habitat", "Habitat de Ar", "air"),
            ("earth_habitat", "Habitat de Terra", "earth"),
            ("lightning_habitat", "Habitat de Raio", "lightning"),
            ("light_habitat", "Habitat de Luz", "light"),
            ("shadow_habitat", "Habitat de Sombra", "shadow"),
        ];

        for (id, name, el) in elements {
            catalog.register(HabitatType {
                id: id.to_string(),
                name: name.to_string(),
                supported_elements: vec![ElementId::new(el)],
                width: 3,
                height: 3,
                tiers: vec![
                    HabitatTierInfo {
                        tier: 1,
                        max_creatures: 2,
                        gold_capacity: 500,
                        upgrade_cost_gold: 0,
                    },
                    HabitatTierInfo {
                        tier: 2,
                        max_creatures: 3,
                        gold_capacity: 2500,
                        upgrade_cost_gold: 1000,
                    },
                    HabitatTierInfo {
                        tier: 3,
                        max_creatures: 4,
                        gold_capacity: 10000,
                        upgrade_cost_gold: 5000,
                    },
                ],
            });
        }

        catalog
    }

    pub fn register(&mut self, hab_type: HabitatType) {
        self.types.insert(hab_type.id.clone(), hab_type);
    }

    pub fn get(&self, id: &str) -> Option<&HabitatType> {
        self.types.get(id)
    }

    pub fn count(&self) -> usize {
        self.types.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_placement_and_collision() {
        let mut grid = IslandGrid::new(10, 10);
        let catalog = HabitatCatalog::with_default_habitats();
        let fire_type = catalog.get("fire_habitat").unwrap();

        let mut hab1 = HabitatInstance::new(1, "fire_habitat", 0, 0);
        assert!(grid.place_habitat(&hab1, fire_type).is_ok());

        // Verifica que as células de (0,0) a (2,2) estão ocupadas
        assert!(grid.is_occupied(0, 0));
        assert!(grid.is_occupied(2, 2));
        assert!(!grid.is_occupied(3, 3));

        // Tentar colocar outro habitat em colisão direta deve falhar
        let hab2 = HabitatInstance::new(2, "fire_habitat", 1, 1);
        assert!(matches!(
            grid.place_habitat(&hab2, fire_type),
            Err(HabitatError::TileOccupied { .. })
        ));

        // Mover hab1 para posição livre (4, 4) deve ter sucesso
        assert!(grid.move_habitat(&mut hab1, fire_type, 4, 4).is_ok());
        assert!(!grid.is_occupied(0, 0));
        assert!(grid.is_occupied(4, 4));
    }

    #[test]
    fn test_creature_assignment_and_gold_generation() {
        let hab_catalog = HabitatCatalog::with_default_habitats();
        let species_catalog = SpeciesCatalog::with_starter_species();
        let fire_type = hab_catalog.get("fire_habitat").unwrap();

        let mut habitat = HabitatInstance::new(1, "fire_habitat", 0, 0);
        let mut creatures = HashMap::new();

        // Cria Salamandra de Fogo
        let pyro = species_catalog.get("pyro_salamander").unwrap();
        let pyro_inst = CreatureInstance::new(10, &pyro.id);
        creatures.insert(pyro_inst.instance_id, pyro_inst.clone());

        // Cria Serpente de Água (elemento incompatível com habitat de Fogo)
        let water = species_catalog.get("tide_serpent").unwrap();
        let water_inst = CreatureInstance::new(20, &water.id);
        creatures.insert(water_inst.instance_id, water_inst.clone());

        // Tentar adicionar Serpente de Água no habitat de Fogo DEVE falhar
        let err = habitat.add_creature(&water_inst, fire_type, &species_catalog);
        assert!(matches!(err, Err(HabitatError::ElementNotSupported { .. })));

        // Adicionar Salamandra de Fogo DEVE suceder
        assert!(habitat.add_creature(&pyro_inst, fire_type, &species_catalog).is_ok());

        // Simula 1 hora (3600 segundos) de geração passiva
        habitat.generate_gold(3600.0, &creatures, &species_catalog, fire_type);
        assert!(habitat.accumulated_gold > 0);
        assert!(habitat.accumulated_gold <= fire_type.tiers[0].gold_capacity);

        // Coleta de ouro
        let collected = habitat.collect_gold();
        assert!(collected > 0);
        assert_eq!(habitat.accumulated_gold, 0);
    }
}
