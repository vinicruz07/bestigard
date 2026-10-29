//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{App, EngineConfig, GameState};
use game_core::{
    Bestiary, CropCatalog, CreatureInstance, ElementRegistry, FarmInstance, FeedingSystem,
    HabitatCatalog, HabitatInstance, HatcheryInstance, IncubationSpeedModifier, IslandGrid,
    MilestoneCatalog, ModifierSource, SpeciesCatalog,
};
use std::collections::HashMap;

pub struct BestigardGame {
    elements: ElementRegistry,
    catalog: SpeciesCatalog,
    habitats_catalog: HabitatCatalog,
    crops_catalog: CropCatalog,
    milestones_catalog: MilestoneCatalog,
    bestiary: Bestiary,
    grid: IslandGrid,
    habitats: Vec<HabitatInstance>,
    farms: Vec<FarmInstance>,
    hatchery: HatcheryInstance,
    creatures: HashMap<u64, CreatureInstance>,
    player_gold: u64,
    player_food: u64,
    frames_rendered: u64,
    ticks_processed: u64,
}

impl BestigardGame {
    pub fn new() -> Self {
        Self {
            elements: ElementRegistry::with_default_elements(),
            catalog: SpeciesCatalog::with_starter_species(),
            habitats_catalog: HabitatCatalog::with_default_habitats(),
            crops_catalog: CropCatalog::with_default_crops(),
            milestones_catalog: MilestoneCatalog::with_default_milestones(),
            bestiary: Bestiary::new(),
            grid: IslandGrid::new(16, 16),
            habitats: Vec::new(),
            farms: Vec::new(),
            hatchery: HatcheryInstance::new(1, 3), // Ninho com 3 slots
            creatures: HashMap::new(),
            player_gold: 500,
            player_food: 0,
            frames_rendered: 0,
            ticks_processed: 0,
        }
    }
}

impl Default for BestigardGame {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState for BestigardGame {
    fn init(&mut self) {
        println!(
            "BestigardGame inicializado (Core v{}) - {} elementos, {} espécies, {} culturas, {} marcos",
            game_core::core_version(),
            self.elements.count(),
            self.catalog.count(),
            self.crops_catalog.count(),
            self.milestones_catalog.count()
        );

        // 1. Constrói o Habitat de Fogo na posição (2, 2)
        let fire_hab_type = self.habitats_catalog.get("fire_habitat").expect("Fire habitat should exist");
        let mut habitat = HabitatInstance::new(1, "fire_habitat", 2, 2);
        self.grid.place_habitat(&habitat, fire_hab_type).expect("Habitat placement should succeed");

        // 2. Aloca a criatura inicial (Salamandra Ígnea) no habitat e registra no Bestiário
        if let Some(pyro) = self.catalog.get("pyro_salamander") {
            let mut creature = CreatureInstance::new(1, &pyro.id);
            creature.nickname = Some("Ignis".to_string());
            creature.habitat_id = Some(habitat.instance_id);

            habitat.add_creature(&creature, fire_hab_type, &self.catalog).expect("Should add creature to habitat");
            self.bestiary.record_discovery(&pyro.id, 100);
            self.creatures.insert(creature.instance_id, creature);
        }
        self.habitats.push(habitat);

        // 3. Constrói a Fazenda de Comida na posição (6, 2) e planta Fruta Silvestre
        let mut farm = FarmInstance::new(100, 6, 2);
        let berry = self.crops_catalog.get("wild_berry").expect("Wild berry crop should exist");
        farm.plant_crop(berry, &mut self.player_gold).expect("Crop planting should succeed");
        self.farms.push(farm);

        // 4. Adiciona um ovo no ninho (Besta Vulcânica - híbrido)
        if let Some(volcano) = self.catalog.get("volcano_beast") {
            let egg_id = self.hatchery.add_egg(volcano).expect("Egg addition should succeed");
            println!(
                "Ninho: Ovo #{} de '{}' depositado (duração base: {}s)",
                egg_id, volcano.name, volcano.incubation_time_secs
            );

            // Equipa modificador genérico de velocidade (2.5x de velocidade)
            self.hatchery.add_modifier(IncubationSpeedModifier::permanent(
                "primal_heat_amulet",
                "Amuleto do Calor Primordial",
                ModifierSource::Amulet("primal_heat_amulet".to_string()),
                2.5,
            ));
        }
    }

    fn fixed_update(&mut self, fixed_dt: f32) {
        self.ticks_processed += 1;

        // Atualiza a produção de ouro dos habitats
        let fire_type = self.habitats_catalog.get("fire_habitat").unwrap();
        for hab in &mut self.habitats {
            hab.generate_gold(fixed_dt as f64 * 60.0, &self.creatures, &self.catalog, fire_type);
        }

        // Atualiza o crescimento das fazendas
        for farm in &mut self.farms {
            farm.update_growth(fixed_dt as f64 * 30.0);
        }

        // Atualiza o progresso da incubadora de ovos
        self.hatchery.update_incubation(fixed_dt as f64 * 30.0);
    }

    fn update(&mut self, _dt: f32) {
        // Atualizações visuais de apresentação
    }

    fn render(&mut self, _alpha: f32) {
        self.frames_rendered += 1;
    }

    fn on_pause(&mut self) {
        println!("Bestigard suspenso (background mobile)");
    }

    fn on_resume(&mut self) {
        println!("Bestigard retomado (foreground mobile)");
    }
}

fn main() {
    let config = EngineConfig::default();
    println!(
        "Iniciando {} - Resolução alvo: {}x{}, UPS: {}",
        config.title, config.width, config.height, config.target_ups
    );

    let mut app = App::new(config, BestigardGame::default());

    // Executa passos do loop para simular o tempo de fazenda e incubação
    for _ in 0..40 {
        app.step(1.0 / 60.0);
    }

    let state = app.state_mut();

    // 1. Colheita de comida
    let harvested_food = state.farms[0].harvest().expect("Harvest should succeed");
    state.player_food += harvested_food;
    println!("Colheita concluída: +{} de comida.", harvested_food);

    // 2. Alimentação da Salamandra
    if let Some(creature) = state.creatures.get_mut(&1) {
        let feed_res = FeedingSystem::feed_creature(creature, &mut state.player_food).expect("Feeding should succeed");
        state.bestiary.record_level(&creature.species_id, feed_res.new_level);
        println!(
            "Criatura '{}' alimentada. Nível: {}, Estágio: {:?}",
            creature.nickname.as_deref().unwrap_or(""),
            feed_res.new_level,
            feed_res.new_stage
        );
    }

    // 3. Eclosão do ovo da Besta Vulcânica
    let new_creature = state
        .hatchery
        .hatch_egg(1, 2, &state.catalog)
        .expect("Egg should hatch successfully");

    state.bestiary.record_discovery(&new_creature.species_id, 200);

    println!(
        "ECLOSÃO! Nasceu a criatura #{} ('{}') estágio {:?}",
        new_creature.instance_id,
        state.catalog.get(&new_creature.species_id).unwrap().name,
        new_creature.stage
    );

    // Aloca a nova criatura no habitat de Fogo
    let fire_hab_type = state.habitats_catalog.get("fire_habitat").unwrap();
    state.habitats[0]
        .add_creature(&new_creature, fire_hab_type, &state.catalog)
        .expect("Newborn should enter fire habitat");
    state.creatures.insert(new_creature.instance_id, new_creature);

    // 4. Bestiário e Recompensas de Marcos
    let completion = state.bestiary.completion_percentage(&state.catalog);
    println!(
        "Bestiário: {}/{} espécies descobertas ({:.1}% concluído)",
        state.bestiary.discovered_count(),
        state.catalog.count(),
        completion
    );

    // Reivindica o marco "Despertar das Bestas" (first_discovery)
    let reward = state
        .bestiary
        .claim_milestone(
            "first_discovery",
            &state.milestones_catalog,
            &state.catalog,
            &mut state.player_gold,
            &mut state.player_food,
        )
        .expect("Milestone claim should succeed");

    println!(
        "MARCO REIVINDICADO! 'Despertar das Bestas': +{} ouro, +{} comida! Saldo atual: {} ouro, {} comida.",
        reward.gold, reward.food, state.player_gold, state.player_food
    );
}
