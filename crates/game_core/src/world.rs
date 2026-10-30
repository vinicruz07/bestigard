//! Estado agregado central e agregador de mundo do santuário de Bestigard (SanctuaryWorld).
#![forbid(unsafe_code)]

use crate::bestiary::{Bestiary, MilestoneCatalog, MilestoneReward};
use crate::breeding::{BreedingEngine, BreedingError, BreedingResult};
use crate::creature::{CreatureInstance, CreatureSpecies, SpeciesCatalog};
use crate::farming::{CropType, FarmError, FarmInstance, FarmState, FeedResult, FeedingError, FeedingSystem};
use crate::habitat::{HabitatCatalog, HabitatError, HabitatInstance, IslandGrid};
use crate::incubation::{HatcheryError, HatcheryInstance};
use crate::obstacle::{Obstacle, ObstacleKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Erros consolidados das operações de mundo no santuário.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldError {
    Habitat(HabitatError),
    Farm(FarmError),
    Hatchery(HatcheryError),
    Feeding(FeedingError),
    Breeding(BreedingError),
    CreatureNotFound(u64),
    FarmNotFound(u64),
    SpeciesNotFound(String),
    SpeciesNotPurchasable(String),
    ObstacleNotFound(u64),
    InsufficientGold { required: u64, available: u64 },
}

impl From<HabitatError> for WorldError {
    fn from(err: HabitatError) -> Self {
        Self::Habitat(err)
    }
}

impl From<FarmError> for WorldError {
    fn from(err: FarmError) -> Self {
        Self::Farm(err)
    }
}

impl From<HatcheryError> for WorldError {
    fn from(err: HatcheryError) -> Self {
        Self::Hatchery(err)
    }
}

impl From<FeedingError> for WorldError {
    fn from(err: FeedingError) -> Self {
        Self::Feeding(err)
    }
}

impl From<BreedingError> for WorldError {
    fn from(err: BreedingError) -> Self {
        Self::Breeding(err)
    }
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Habitat(e) => write!(f, "Erro de habitat: {}", e),
            Self::Farm(e) => write!(f, "Erro de fazenda: {}", e),
            Self::Hatchery(e) => write!(f, "Erro de incubadora: {}", e),
            Self::Feeding(e) => write!(f, "Erro de alimentação: {}", e),
            Self::Breeding(e) => write!(f, "Erro de cruzamento: {}", e),
            Self::CreatureNotFound(id) => write!(f, "Criatura #{} não encontrada no santuário", id),
            Self::FarmNotFound(id) => write!(f, "Fazenda #{} não encontrada", id),
            Self::SpeciesNotFound(id) => write!(f, "Espécie '{}' não encontrada no catálogo", id),
            Self::SpeciesNotPurchasable(id) => {
                write!(f, "O ovo da espécie '{}' não pode ser comprado diretamente (requer cruzamento)", id)
            }
            Self::ObstacleNotFound(id) => write!(f, "Obstáculo #{} não encontrado", id),
            Self::InsufficientGold { required, available } => {
                write!(f, "Ouro insuficiente: necessário {}, disponível {}", required, available)
            }
        }
    }
}

impl std::error::Error for WorldError {}

/// Relatório do progresso calculado durante ausência do jogador (offline catch-up).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineProgressReport {
    pub elapsed_applied_secs: f64,
    pub gold_generated: u64,
    pub farms_matured: usize,
    pub eggs_ready: usize,
}

/// Limite máximo justo de simulação offline (8 horas = 28.800 segundos).
pub const MAX_OFFLINE_SECONDS: f64 = 8.0 * 3600.0;

/// O estado completo, unificado e serializável do mundo e santuário do jogador.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctuaryWorld {
    pub player_name: String,
    pub player_level: u32,
    /// Experiência acumulada do jogador rumo ao próximo nível do santuário.
    #[serde(default)]
    pub player_xp: u64,
    pub gold: u64,
    pub food: u64,
    pub island_grid: IslandGrid,
    pub habitats: HashMap<u64, HabitatInstance>,
    pub farms: HashMap<u64, FarmInstance>,
    pub hatchery: HatcheryInstance,
    pub creatures: HashMap<u64, CreatureInstance>,
    pub bestiary: Bestiary,
    /// Obstáculos naturais da ilha (compatível com savegames anteriores sem este campo).
    #[serde(default)]
    pub obstacles: HashMap<u64, Obstacle>,
    pub next_id_counter: u64,
    pub total_simulated_seconds: f64,
}

/// Custo em ouro para construir uma fazenda agrícola.
pub const FARM_BUILD_COST_GOLD: u64 = 50;

impl SanctuaryWorld {
    /// Inicializa um novo santuário com o grid e recursos configurados.
    pub fn new(
        player_name: impl Into<String>,
        grid_width: u32,
        grid_height: u32,
        starting_gold: u64,
        starting_food: u64,
    ) -> Self {
        Self {
            player_name: player_name.into(),
            player_level: 1,
            player_xp: 0,
            gold: starting_gold,
            food: starting_food,
            island_grid: IslandGrid::new(grid_width, grid_height),
            habitats: HashMap::new(),
            farms: HashMap::new(),
            hatchery: HatcheryInstance::new(1, 3),
            creatures: HashMap::new(),
            bestiary: Bestiary::new(),
            obstacles: HashMap::new(),
            next_id_counter: 10,
            total_simulated_seconds: 0.0,
        }
    }

    /// Cria uma configuração inicial padrão com uma ilha 16x16, Habitat de Fogo e Salamandra Ígnea.
    pub fn with_starter_setup(
        player_name: impl Into<String>,
        catalog: &SpeciesCatalog,
        hab_catalog: &HabitatCatalog,
    ) -> Result<Self, WorldError> {
        let mut world = Self::new(player_name, 16, 16, 500, 100);

        // 1. Constrói Habitat de Fogo em (2, 2)
        let hab_type = hab_catalog.get("fire_habitat").ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound { habitat_id: 0 })
        })?;

        let hab_id = world.next_id();
        let mut habitat = HabitatInstance::new(hab_id, "fire_habitat", 2, 2);
        world.island_grid.place_habitat(&habitat, hab_type)?;

        // 2. Cria Salamandra Ígnea inicial e aloca no habitat
        if let Some(pyro) = catalog.get("pyro_salamander") {
            let creature_id = world.next_id();
            let mut creature = CreatureInstance::new(creature_id, &pyro.id);
            creature.nickname = Some("Ignis".to_string());
            creature.habitat_id = Some(hab_id);

            habitat.add_creature(&creature, hab_type, catalog)?;
            world.bestiary.record_discovery(&pyro.id, 0);
            world.creatures.insert(creature_id, creature);
        }

        world.habitats.insert(hab_id, habitat);

        // 3. Espalha obstáculos naturais pela ilha (fora da área inicial de habitat/fazenda)
        let starter_obstacles = [
            (ObstacleKind::SmallRock, 10, 3),
            (ObstacleKind::Tree, 5, 8),
            (ObstacleKind::LargeRock, 12, 6),
            (ObstacleKind::Bush, 8, 10),
            (ObstacleKind::Tree, 1, 12),
        ];
        for (kind, x, y) in starter_obstacles {
            world.add_obstacle(kind, x, y)?;
        }

        Ok(world)
    }

    /// XP necessário para o jogador avançar para o próximo nível do santuário.
    pub fn player_xp_for_next_level(&self) -> u64 {
        let lvl = self.player_level.max(1) as f64;
        (100.0 * lvl.powf(1.5)).round() as u64
    }

    /// Adiciona experiência ao jogador e processa subidas de nível do santuário. Retorna quantos níveis subiu.
    pub fn add_player_xp(&mut self, amount: u64) -> u32 {
        self.player_xp += amount;
        let initial_level = self.player_level;

        while self.player_xp >= self.player_xp_for_next_level() {
            self.player_xp -= self.player_xp_for_next_level();
            self.player_level += 1;
        }

        self.player_level - initial_level
    }

    /// Aplica progresso offline baseado no tempo decorrido desde a última sessão, respeitando o limite justo (`MAX_OFFLINE_SECONDS`).
    pub fn apply_offline_progress(
        &mut self,
        elapsed_real_secs: f64,
        catalog: &SpeciesCatalog,
        hab_catalog: &HabitatCatalog,
    ) -> OfflineProgressReport {
        let capped_secs = elapsed_real_secs.clamp(0.0, MAX_OFFLINE_SECONDS);
        if capped_secs <= 0.0 {
            return OfflineProgressReport {
                elapsed_applied_secs: 0.0,
                gold_generated: 0,
                farms_matured: 0,
                eggs_ready: 0,
            };
        }

        let gold_before: u64 = self.habitats.values().map(|h| h.accumulated_gold).sum();
        let growing_before: usize = self
            .farms
            .values()
            .filter(|f| matches!(f.state, FarmState::Growing { .. }))
            .count();
        let ready_before: usize = self
            .farms
            .values()
            .filter(|f| matches!(f.state, FarmState::ReadyToHarvest { .. }))
            .count();

        self.tick(capped_secs, catalog, hab_catalog);

        let gold_after: u64 = self.habitats.values().map(|h| h.accumulated_gold).sum();
        let ready_after: usize = self
            .farms
            .values()
            .filter(|f| matches!(f.state, FarmState::ReadyToHarvest { .. }))
            .count();
        let farms_matured = ready_after.saturating_sub(ready_before).min(growing_before);
        let eggs_ready = self.hatchery.eggs.iter().filter(|e| e.is_ready()).count();

        OfflineProgressReport {
            elapsed_applied_secs: capped_secs,
            gold_generated: gold_after.saturating_sub(gold_before),
            farms_matured,
            eggs_ready,
        }
    }

    /// Posiciona um obstáculo na ilha ocupando as células do grid.
    pub fn add_obstacle(&mut self, kind: ObstacleKind, grid_x: u32, grid_y: u32) -> Result<u64, WorldError> {
        let (w, h) = kind.footprint();
        self.island_grid.can_place(grid_x, grid_y, w, h, None)?;
        let id = self.next_id();
        self.island_grid.occupy_area(grid_x, grid_y, w, h, id)?;
        self.obstacles.insert(id, Obstacle::new(id, kind, grid_x, grid_y));
        Ok(id)
    }

    /// Retorna o obstáculo que cobre a célula `(x, y)`, se houver.
    pub fn obstacle_at(&self, x: u32, y: u32) -> Option<&Obstacle> {
        self.obstacles.values().find(|o| o.covers(x, y))
    }

    /// Limpa um obstáculo mediante pagamento em ouro, liberando suas células. Retorna o custo pago.
    pub fn clear_obstacle(&mut self, obstacle_id: u64) -> Result<u64, WorldError> {
        let obstacle = self
            .obstacles
            .get(&obstacle_id)
            .ok_or(WorldError::ObstacleNotFound(obstacle_id))?
            .clone();
        let cost = obstacle.kind.clear_cost_gold();
        if self.gold < cost {
            return Err(WorldError::InsufficientGold { required: cost, available: self.gold });
        }
        let (w, h) = obstacle.kind.footprint();
        self.gold -= cost;
        self.island_grid.release_area(obstacle.grid_x, obstacle.grid_y, w, h, obstacle_id);
        self.obstacles.remove(&obstacle_id);
        self.add_player_xp(15);
        Ok(cost)
    }

    /// Verifica se uma área retangular da ilha está livre para construção.
    pub fn can_place_building(&self, grid_x: u32, grid_y: u32, w: u32, h: u32) -> Result<(), WorldError> {
        self.island_grid.can_place(grid_x, grid_y, w, h, None)?;
        Ok(())
    }

    /// Gera um ID único e incremental para novas entidades.
    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id_counter;
        self.next_id_counter += 1;
        id
    }

    /// Avança a simulação contínua do mundo (geração de ouro, crescimento de colheitas e incubação de ovos).
    pub fn tick(
        &mut self,
        dt_secs: f64,
        catalog: &SpeciesCatalog,
        hab_catalog: &HabitatCatalog,
    ) {
        if dt_secs <= 0.0 {
            return;
        }

        self.total_simulated_seconds += dt_secs;

        // 1. Atualiza habitats (geração de ouro das criaturas)
        for hab in self.habitats.values_mut() {
            if let Some(hab_type) = hab_catalog.get(&hab.type_id) {
                hab.generate_gold(dt_secs, &self.creatures, catalog, hab_type);
            }
        }

        // 2. Atualiza fazendas (tempo de maturação das culturas)
        for farm in self.farms.values_mut() {
            farm.update_growth(dt_secs);
        }

        // 3. Atualiza ninho de incubação (tempo de choca dos ovos com aceleradores)
        self.hatchery.update_incubation(dt_secs);
    }

    /// Constrói um novo habitat consumindo custo e validando espaço no grid.
    pub fn build_habitat(
        &mut self,
        type_id: &str,
        grid_x: u32,
        grid_y: u32,
        gold_cost: u64,
        hab_catalog: &HabitatCatalog,
    ) -> Result<u64, WorldError> {
        if self.gold < gold_cost {
            return Err(WorldError::InsufficientGold {
                required: gold_cost,
                available: self.gold,
            });
        }

        let hab_type = hab_catalog.get(type_id).ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound { habitat_id: 0 })
        })?;

        let hab_id = self.next_id();
        let habitat = HabitatInstance::new(hab_id, type_id, grid_x, grid_y);
        self.island_grid.place_habitat(&habitat, hab_type)?;

        self.gold -= gold_cost;
        self.habitats.insert(hab_id, habitat);
        self.add_player_xp(30);

        Ok(hab_id)
    }

    /// Retorna o custo em ouro para evoluir um habitat para o próximo tier (`None` se já estiver no tier máximo).
    pub fn habitat_upgrade_cost(
        &self,
        habitat_id: u64,
        hab_catalog: &HabitatCatalog,
    ) -> Option<u64> {
        let hab = self.habitats.get(&habitat_id)?;
        let hab_type = hab_catalog.get(&hab.type_id)?;
        hab_type
            .get_tier_info(hab.tier + 1)
            .map(|t| t.upgrade_cost_gold)
    }

    /// Evolui um habitat para o próximo tier descontando ouro e expandindo capacidade de criaturas e cofre.
    pub fn upgrade_habitat(
        &mut self,
        habitat_id: u64,
        hab_catalog: &HabitatCatalog,
    ) -> Result<u32, WorldError> {
        let hab = self.habitats.get(&habitat_id).ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound { habitat_id })
        })?;
        let hab_type = hab_catalog.get(&hab.type_id).ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound { habitat_id })
        })?;

        let next_tier = hab.tier + 1;
        let next_info = hab_type
            .get_tier_info(next_tier)
            .ok_or(WorldError::Habitat(HabitatError::MaxTierReached))?;

        let cost = next_info.upgrade_cost_gold;
        if self.gold < cost {
            return Err(WorldError::InsufficientGold {
                required: cost,
                available: self.gold,
            });
        }

        let hab_mut = self.habitats.get_mut(&habitat_id).unwrap();
        hab_mut.upgrade(hab_type)?;
        self.gold -= cost;
        self.add_player_xp(50);

        Ok(next_tier)
    }

    /// Constrói uma nova fazenda agrícola consumindo ouro e posicionando no grid.
    pub fn build_farm(
        &mut self,
        grid_x: u32,
        grid_y: u32,
        gold_cost: u64,
    ) -> Result<u64, WorldError> {
        if self.gold < gold_cost {
            return Err(WorldError::InsufficientGold {
                required: gold_cost,
                available: self.gold,
            });
        }

        let farm_id = self.next_id();
        let farm = FarmInstance::new(farm_id, grid_x, grid_y);

        // Valida espaço de 2x2 no grid e reserva as células
        self.island_grid.can_place(grid_x, grid_y, farm.width, farm.height, None)?;
        self.island_grid.occupy_area(grid_x, grid_y, farm.width, farm.height, farm_id)?;

        self.gold -= gold_cost;
        self.farms.insert(farm_id, farm);
        self.add_player_xp(20);

        Ok(farm_id)
    }

    /// Planta uma cultura agrícola em uma fazenda específica.
    pub fn plant_crop(&mut self, farm_id: u64, crop: &CropType) -> Result<(), WorldError> {
        let farm = self.farms.get_mut(&farm_id).ok_or(WorldError::FarmNotFound(farm_id))?;
        farm.plant_crop(crop, &mut self.gold)?;
        Ok(())
    }

    /// Colhe os alimentos produzidos em uma fazenda e adiciona ao estoque do jogador.
    pub fn harvest_farm(&mut self, farm_id: u64) -> Result<u64, WorldError> {
        let farm = self.farms.get_mut(&farm_id).ok_or(WorldError::FarmNotFound(farm_id))?;
        let food_yield = farm.harvest()?;
        self.food += food_yield;
        self.add_player_xp(10);
        Ok(food_yield)
    }

    /// Coleta todo o ouro acumulado em um habitat específico.
    pub fn collect_habitat_gold(&mut self, habitat_id: u64) -> Result<u64, WorldError> {
        let hab = self.habitats.get_mut(&habitat_id).ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound { habitat_id })
        })?;

        let collected = hab.collect_gold();
        self.gold += collected;
        Ok(collected)
    }

    /// Coleta o ouro acumulado de todos os habitats da ilha em uma única ação rápida.
    pub fn collect_all_gold(&mut self) -> u64 {
        let mut total = 0u64;
        for hab in self.habitats.values_mut() {
            total += hab.collect_gold();
        }
        self.gold += total;
        total
    }

    /// Alimenta uma criatura consumindo estoque de comida e atualizando o Bestiário.
    pub fn feed_creature(&mut self, creature_id: u64) -> Result<FeedResult, WorldError> {
        let creature = self.creatures.get_mut(&creature_id).ok_or(WorldError::CreatureNotFound(creature_id))?;
        let res = FeedingSystem::feed_creature(creature, &mut self.food)?;
        let species_id = creature.species_id.clone();
        self.bestiary.record_level(&species_id, res.new_level);
        self.add_player_xp(15);
        Ok(res)
    }

    /// Renomeia uma criatura (ou restaura o nome da espécie caso o apelido seja vazio).
    pub fn rename_creature(
        &mut self,
        creature_id: u64,
        new_nickname: impl Into<String>,
    ) -> Result<(), WorldError> {
        let creature = self
            .creatures
            .get_mut(&creature_id)
            .ok_or(WorldError::CreatureNotFound(creature_id))?;
        let raw = new_nickname.into();
        let trimmed = raw.trim();
        creature.nickname = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
        Ok(())
    }

    /// Transfere uma criatura para outro habitat compatível que possua capacidade livre.
    pub fn relocate_creature(
        &mut self,
        creature_id: u64,
        target_habitat_id: u64,
        catalog: &SpeciesCatalog,
        hab_catalog: &HabitatCatalog,
    ) -> Result<(), WorldError> {
        let creature_snapshot = self
            .creatures
            .get(&creature_id)
            .ok_or(WorldError::CreatureNotFound(creature_id))?
            .clone();

        if creature_snapshot.habitat_id == Some(target_habitat_id) {
            return Ok(());
        }

        let target_type_id = self
            .habitats
            .get(&target_habitat_id)
            .ok_or(WorldError::Habitat(HabitatError::HabitatNotFound {
                habitat_id: target_habitat_id,
            }))?
            .type_id
            .clone();

        let target_type = hab_catalog.get(&target_type_id).ok_or(WorldError::Habitat(
            HabitatError::HabitatNotFound {
                habitat_id: target_habitat_id,
            },
        ))?;

        // 1. Tenta adicionar no habitat de destino (valida afinidade elemental e capacidade)
        {
            let target_hab = self.habitats.get_mut(&target_habitat_id).unwrap();
            target_hab.add_creature(&creature_snapshot, target_type, catalog)?;
        }

        // 2. Remove do habitat anterior, se houver
        if let Some(old_hab_id) = creature_snapshot.habitat_id {
            if let Some(old_hab) = self.habitats.get_mut(&old_hab_id) {
                old_hab.remove_creature(creature_id);
            }
        }

        // 3. Atualiza vínculo na instância da criatura
        if let Some(creature_mut) = self.creatures.get_mut(&creature_id) {
            creature_mut.habitat_id = Some(target_habitat_id);
        }

        Ok(())
    }

    ///Deposita um novo ovo no ninho da incubadora.
    pub fn deposit_egg(&mut self, species: &CreatureSpecies) -> Result<u64, WorldError> {
        let egg_id = self.hatchery.add_egg(species)?;
        Ok(egg_id)
    }

    /// Compra um ovo mono-elemental básico na loja usando ouro e deposita no ninho.
    pub fn buy_egg(
        &mut self,
        species_id: &str,
        catalog: &SpeciesCatalog,
    ) -> Result<u64, WorldError> {
        let species = catalog
            .get(species_id)
            .ok_or_else(|| WorldError::SpeciesNotFound(species_id.to_string()))?;

        let price = species
            .egg_shop_price_gold()
            .ok_or_else(|| WorldError::SpeciesNotPurchasable(species_id.to_string()))?;

        if self.hatchery.eggs.len() >= self.hatchery.max_slots {
            return Err(WorldError::Hatchery(HatcheryError::HatcheryFull {
                max_slots: self.hatchery.max_slots,
            }));
        }

        if self.gold < price {
            return Err(WorldError::InsufficientGold {
                required: price,
                available: self.gold,
            });
        }

        let egg_id = self.hatchery.add_egg(species)?;
        self.gold -= price;
        self.add_player_xp(15);

        Ok(egg_id)
    }

    /// Vende ou libera um ovo do ninho em troca de ouro baseado na raridade.
    pub fn sell_egg(
        &mut self,
        egg_id: u64,
        catalog: &SpeciesCatalog,
    ) -> Result<u64, WorldError> {
        let reward = self.hatchery.sell_or_release_egg(egg_id, catalog)?;
        self.gold += reward;
        Ok(reward)
    }

    /// Realiza o cruzamento genético determinístico entre duas criaturas e deposita o ovo resultante no ninho.
    pub fn breed_creatures(
        &mut self,
        parent_a_id: u64,
        parent_b_id: u64,
        catalog: &SpeciesCatalog,
        seed: u64,
    ) -> Result<BreedingResult, WorldError> {
        let parent_a = self
            .creatures
            .get(&parent_a_id)
            .ok_or(WorldError::CreatureNotFound(parent_a_id))?;
        let parent_b = self
            .creatures
            .get(&parent_b_id)
            .ok_or(WorldError::CreatureNotFound(parent_b_id))?;

        let species_a = catalog.get(&parent_a.species_id).ok_or_else(|| {
            WorldError::SpeciesNotFound(parent_a.species_id.clone())
        })?;
        let species_b = catalog.get(&parent_b.species_id).ok_or_else(|| {
            WorldError::SpeciesNotFound(parent_b.species_id.clone())
        })?;

        let engine = BreedingEngine::new();
        let result = engine.breed(parent_a, species_a, parent_b, species_b, catalog, seed)?;

        let offspring_species = catalog
            .get(&result.offspring_species_id)
            .ok_or_else(|| WorldError::SpeciesNotFound(result.offspring_species_id.clone()))?;

        // Deposita o ovo automaticamente no ninho de incubação
        self.deposit_egg(offspring_species)?;
        self.add_player_xp(25);

        Ok(result)
    }

    /// Choca um ovo pronto, registra no Bestiário e aloca no habitat de destino.
    pub fn hatch_egg(
        &mut self,
        egg_id: u64,
        target_habitat_id: u64,
        catalog: &SpeciesCatalog,
        hab_catalog: &HabitatCatalog,
    ) -> Result<u64, WorldError> {
        // Valida primeiro o ovo e o habitat de destino para não consumir o ovo em caso de erro de habitat
        let egg = self
            .hatchery
            .eggs
            .iter()
            .find(|e| e.egg_id == egg_id)
            .ok_or(WorldError::Hatchery(HatcheryError::EggNotFound { egg_id }))?;

        if !egg.is_ready() {
            return Err(WorldError::Hatchery(HatcheryError::EggNotReady {
                remaining_secs: egg.remaining_secs(),
            }));
        }

        let hab_check = self.habitats.get(&target_habitat_id).ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound {
                habitat_id: target_habitat_id,
            })
        })?;
        let hab_type = hab_catalog.get(&hab_check.type_id).ok_or_else(|| {
            WorldError::Habitat(HabitatError::HabitatNotFound {
                habitat_id: target_habitat_id,
            })
        })?;

        let preview_id = self.next_id_counter;
        let preview_creature = CreatureInstance::new(preview_id, &egg.species_id);
        let mut hab_preview = hab_check.clone();
        hab_preview.add_creature(&preview_creature, hab_type, catalog)?;

        // Tudo validado: consome o ovo e registra a criatura nascida
        let new_id = self.next_id();
        let mut new_creature = self.hatchery.hatch_egg(egg_id, new_id, catalog)?;
        new_creature.habitat_id = Some(target_habitat_id);

        let hab = self.habitats.get_mut(&target_habitat_id).unwrap();
        hab.add_creature(&new_creature, hab_type, catalog)?;

        self.bestiary
            .record_discovery(&new_creature.species_id, self.total_simulated_seconds as u64);
        self.creatures.insert(new_id, new_creature);
        self.add_player_xp(40);

        Ok(new_id)
    }

    /// Reivindica um marco do Bestiário, adicionando ouro e comida diretamente ao jogador.
    pub fn claim_milestone(
        &mut self,
        milestone_id: &str,
        milestone_catalog: &MilestoneCatalog,
        catalog: &SpeciesCatalog,
    ) -> Result<MilestoneReward, crate::bestiary::BestiaryError> {
        let reward = self.bestiary.claim_milestone(
            milestone_id,
            milestone_catalog,
            catalog,
            &mut self.gold,
            &mut self.food,
        )?;
        self.add_player_xp(50);
        Ok(reward)
    }

    /// Serializa todo o estado do mundo para JSON (preparação de savegame).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Reconstrói o mundo a partir de um JSON serializado.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::farming::CropCatalog;

    #[test]
    fn test_world_lifecycle_and_serialization() {
        let species_catalog = SpeciesCatalog::with_starter_species();
        let hab_catalog = HabitatCatalog::with_default_habitats();
        let crop_catalog = CropCatalog::with_default_crops();

        let mut world = SanctuaryWorld::with_starter_setup(
            "Mestre das Bestas",
            &species_catalog,
            &hab_catalog,
        ).expect("Starter setup should succeed");

        assert_eq!(world.gold, 500);
        assert_eq!(world.food, 100);
        assert_eq!(world.creatures.len(), 1);
        assert_eq!(world.habitats.len(), 1);

        // Constrói uma fazenda em (6, 2)
        let farm_id = world.build_farm(6, 2, 50).expect("Should build farm");
        assert_eq!(world.gold, 450);

        // Planta fruta silvestre
        let berry = crop_catalog.get("wild_berry").unwrap();
        world.plant_crop(farm_id, berry).expect("Should plant crop");
        assert_eq!(world.gold, 420); // 450 - 30

        // Avança o tempo do mundo (15 segundos)
        world.tick(15.0, &species_catalog, &hab_catalog);

        // Colheita
        let harvested = world.harvest_farm(farm_id).expect("Should harvest");
        assert_eq!(harvested, 60);
        assert_eq!(world.food, 160); // 100 + 60

        // Alimenta a criatura Ignis
        let creature_id = world.creatures.keys().next().copied().unwrap();
        let feed_res = world.feed_creature(creature_id).expect("Should feed creature");
        assert_eq!(feed_res.food_spent, 20);
        assert_eq!(world.food, 140); // 160 - 20

        // Coleta ouro acumulado
        let gold_collected = world.collect_all_gold();
        assert!(world.gold >= 420 + gold_collected);

        // Serializa para JSON e desserializa de volta
        let json_save = world.to_json().expect("Should serialize world to JSON");
        assert!(!json_save.is_empty());

        let loaded_world = SanctuaryWorld::from_json(&json_save).expect("Should deserialize world");
        assert_eq!(loaded_world.player_name, "Mestre das Bestas");
        assert_eq!(loaded_world.creatures.len(), world.creatures.len());
        assert_eq!(loaded_world.gold, world.gold);
        assert_eq!(loaded_world.food, world.food);
    }

    #[test]
    fn test_world_breeding_lifecycle() {
        let species_catalog = SpeciesCatalog::with_starter_species();
        let hab_catalog = HabitatCatalog::with_default_habitats();

        let mut world = SanctuaryWorld::with_starter_setup(
            "Criador Lendário",
            &species_catalog,
            &hab_catalog,
        ).expect("Starter setup should succeed");

        // Adiciona segunda criatura adulta: Golem Rochoso (Terra)
        let earth_creature_id = world.next_id();
        let mut earth_creature = CreatureInstance::new(earth_creature_id, "stone_golem");
        earth_creature.level = 4;
        earth_creature.stage = crate::creature::GrowthStage::from_level(4);
        world.creatures.insert(earth_creature_id, earth_creature);

        // Promove primeira criatura a adulta (Salamandra Ígnea inicial)
        let fire_creature_id = world
            .creatures
            .keys()
            .find(|&&id| id != earth_creature_id)
            .copied()
            .unwrap();
        let fire_creature = world.creatures.get_mut(&fire_creature_id).unwrap();
        fire_creature.level = 4;
        fire_creature.stage = crate::creature::GrowthStage::from_level(4);

        // Executa cruzamento Fogo + Terra
        let res = world.breed_creatures(
            fire_creature_id,
            earth_creature_id,
            &species_catalog,
            12345,
        ).expect("Breeding should succeed");

        assert_eq!(res.parent_a_id, fire_creature_id);
        assert_eq!(res.parent_b_id, earth_creature_id);
        assert_eq!(world.hatchery.eggs.len(), 1);
        assert!(!res.offspring_species_id.is_empty());
    }

    #[test]
    fn test_building_placement_and_obstacle_clearing() {
        let species_catalog = SpeciesCatalog::with_starter_species();
        let hab_catalog = HabitatCatalog::with_default_habitats();
        let mut world =
            SanctuaryWorld::with_starter_setup("Construtor", &species_catalog, &hab_catalog).unwrap();

        // Obstáculo inicial em (10, 3) bloqueia construção da área
        assert_eq!(world.obstacles.len(), 5);
        assert!(world.can_place_building(10, 3, 1, 1).is_err());
        let rock_id = world.obstacle_at(10, 3).map(|o| o.instance_id).unwrap();

        // Fazenda ocupa suas células no grid (2x2) e impede sobreposição
        let farm_id = world.build_farm(6, 2, FARM_BUILD_COST_GOLD).unwrap();
        assert!(world.island_grid.is_occupied(7, 3));
        assert!(world.build_farm(7, 3, FARM_BUILD_COST_GOLD).is_err());
        assert!(world.farms.contains_key(&farm_id));

        // Limpeza: desconta ouro e libera a célula
        let gold_before = world.gold;
        let cost = world.clear_obstacle(rock_id).unwrap();
        assert_eq!(world.gold, gold_before - cost);
        assert!(world.can_place_building(10, 3, 1, 1).is_ok());
        assert!(world.obstacle_at(10, 3).is_none());
        assert!(matches!(world.clear_obstacle(rock_id), Err(WorldError::ObstacleNotFound(_))));

        // Ouro insuficiente impede a limpeza
        world.gold = 0;
        let tree_id = world.obstacle_at(5, 8).map(|o| o.instance_id).unwrap();
        assert!(matches!(
            world.clear_obstacle(tree_id),
            Err(WorldError::InsufficientGold { .. })
        ));
        assert!(world.obstacles.contains_key(&tree_id));

        // Savegame mantém obstáculos
        let loaded = SanctuaryWorld::from_json(&world.to_json().unwrap()).unwrap();
        assert_eq!(loaded.obstacles.len(), world.obstacles.len());
        assert!(loaded.island_grid.is_occupied(5, 8));
    }

    #[test]
    fn test_habitat_upgrade_and_creature_relocation() {
        let species_catalog = SpeciesCatalog::with_starter_species();
        let hab_catalog = HabitatCatalog::with_default_habitats();
        let mut world =
            SanctuaryWorld::with_starter_setup("Evolucionista", &species_catalog, &hab_catalog)
                .unwrap();

        let starter_hab_id = *world.habitats.keys().next().unwrap();
        let ignis_id = *world.creatures.keys().next().unwrap();

        // Custo de upgrade do Tier 1 para Tier 2 é 1000 ouro; com 500 inicial deve falhar
        assert_eq!(world.habitat_upgrade_cost(starter_hab_id, &hab_catalog), Some(1000));
        assert!(matches!(
            world.upgrade_habitat(starter_hab_id, &hab_catalog),
            Err(WorldError::InsufficientGold { required: 1000, available: 500 })
        ));

        // Com saldo suficiente, sobe para Tier 2 (1000) e Tier 3 (5000)
        world.gold = 7000;
        let tier2 = world.upgrade_habitat(starter_hab_id, &hab_catalog).unwrap();
        assert_eq!(tier2, 2);
        assert_eq!(world.gold, 6000);

        let tier3 = world.upgrade_habitat(starter_hab_id, &hab_catalog).unwrap();
        assert_eq!(tier3, 3);
        assert_eq!(world.gold, 1000);
        assert_eq!(world.habitat_upgrade_cost(starter_hab_id, &hab_catalog), None);

        // Tier 4 não existe -> MaxTierReached
        assert!(matches!(
            world.upgrade_habitat(starter_hab_id, &hab_catalog),
            Err(WorldError::Habitat(HabitatError::MaxTierReached))
        ));

        // Constrói segundo Habitat de Fogo e um Habitat de Água
        let fire_hab_2 = world
            .build_habitat("fire_habitat", 6, 2, 300, &hab_catalog)
            .unwrap();
        let water_hab = world
            .build_habitat("water_habitat", 6, 6, 300, &hab_catalog)
            .unwrap();

        // Tentar mover Ignis (Fogo) para Habitat de Água falha e mantém Ignis no habitat original
        assert!(matches!(
            world.relocate_creature(ignis_id, water_hab, &species_catalog, &hab_catalog),
            Err(WorldError::Habitat(HabitatError::ElementNotSupported { .. }))
        ));
        assert_eq!(world.creatures[&ignis_id].habitat_id, Some(starter_hab_id));

        // Mover Ignis para o segundo Habitat de Fogo funciona e atualiza ambos os habitats
        world
            .relocate_creature(ignis_id, fire_hab_2, &species_catalog, &hab_catalog)
            .unwrap();
        assert_eq!(world.creatures[&ignis_id].habitat_id, Some(fire_hab_2));
        assert!(!world.habitats[&starter_hab_id].creature_ids.contains(&ignis_id));
        assert!(world.habitats[&fire_hab_2].creature_ids.contains(&ignis_id));

        // Renomear criatura
        world.rename_creature(ignis_id, "  Chama Real  ").unwrap();
        assert_eq!(
            world.creatures[&ignis_id].nickname.as_deref(),
            Some("Chama Real")
        );
        world.rename_creature(ignis_id, "   ").unwrap();
        assert!(world.creatures[&ignis_id].nickname.is_none());
    }

    #[test]
    fn test_egg_market_and_offline_progress() {
        let species_catalog = SpeciesCatalog::with_starter_species();
        let hab_catalog = HabitatCatalog::with_default_habitats();
        let crop_catalog = CropCatalog::with_default_crops();
        let mut world =
            SanctuaryWorld::with_starter_setup("Mercador", &species_catalog, &hab_catalog).unwrap();

        // Catálogo lista as 7 espécies mono-elementais para compra de ovo
        let shop_eggs = species_catalog.shop_eggs_sorted();
        assert_eq!(shop_eggs.len(), 7);

        // Espécie híbrida não pode ser comprada na loja
        assert!(matches!(
            world.buy_egg("volcano_beast", &species_catalog),
            Err(WorldError::SpeciesNotPurchasable(_))
        ));

        // Compra ovo de Golem Rochoso (Common = 150 ouro) -> permite ter o 2º progenitor!
        let egg_id = world.buy_egg("stone_golem", &species_catalog).unwrap();
        assert_eq!(world.gold, 350);
        assert_eq!(world.hatchery.eggs.len(), 1);

        // Vende um segundo ovo comprado
        let egg2 = world.buy_egg("tide_serpent", &species_catalog).unwrap();
        assert_eq!(world.gold, 200);
        let sold_gold = world.sell_egg(egg2, &species_catalog).unwrap();
        assert!(sold_gold > 0);
        assert_eq!(world.gold, 200 + sold_gold);

        // Constrói fazenda e planta cultura para testar progresso offline
        let farm_id = world.build_farm(6, 2, FARM_BUILD_COST_GOLD).unwrap();
        let wheat = crop_catalog.get("arcane_wheat").unwrap();
        world.gold += 1000;
        world.plant_crop(farm_id, wheat).unwrap();

        // Simula 24h de ausência -> deve ser limitado a MAX_OFFLINE_SECONDS (8h = 28800s)
        let report = world.apply_offline_progress(24.0 * 3600.0, &species_catalog, &hab_catalog);
        assert_eq!(report.elapsed_applied_secs, MAX_OFFLINE_SECONDS);
        assert!(report.gold_generated > 0);
        assert_eq!(report.farms_matured, 1);
        assert_eq!(report.eggs_ready, 1);

        // Constrói habitat de Terra e choca o ovo de Golem Rochoso (verifica habitat_id atribuído)
        let earth_hab = world
            .build_habitat("earth_habitat", 6, 6, 300, &hab_catalog)
            .unwrap();
        let golem_id = world
            .hatch_egg(egg_id, earth_hab, &species_catalog, &hab_catalog)
            .unwrap();
        assert_eq!(world.creatures[&golem_id].habitat_id, Some(earth_hab));

        // Verifica que o XP do jogador acumulou e subiu de nível
        assert!(world.player_level >= 2 || world.player_xp > 0);
    }
}
