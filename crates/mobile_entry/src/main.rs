//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{App, EngineConfig, GameState};
use game_core::{
    BattleSession, Combatant, CropCatalog, ElementRegistry, ElementalChart, HabitatCatalog,
    IncubationSpeedModifier, MilestoneCatalog, ModifierSource, SanctuaryWorld, SpeciesCatalog,
};

pub struct BestigardGame {
    elements: ElementRegistry,
    catalog: SpeciesCatalog,
    habitats_catalog: HabitatCatalog,
    crops_catalog: CropCatalog,
    milestones_catalog: MilestoneCatalog,
    elemental_chart: ElementalChart,
    world: SanctuaryWorld,
    frames_rendered: u64,
    ticks_processed: u64,
}

impl BestigardGame {
    pub fn new() -> Self {
        let catalog = SpeciesCatalog::with_starter_species();
        let habitats_catalog = HabitatCatalog::with_default_habitats();

        let world = SanctuaryWorld::with_starter_setup("Mestre das Bestas", &catalog, &habitats_catalog)
            .expect("Starter world setup should succeed");

        Self {
            elements: ElementRegistry::with_default_elements(),
            catalog,
            habitats_catalog,
            crops_catalog: CropCatalog::with_default_crops(),
            milestones_catalog: MilestoneCatalog::with_default_milestones(),
            elemental_chart: ElementalChart::with_default_affinities(),
            world,
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
            "BestigardGame inicializado (Core v{}) - Perfil: '{}' ({} elementos registrados), Saldo: {} ouro, {} comida",
            game_core::core_version(),
            self.world.player_name,
            self.elements.count(),
            self.world.gold,
            self.world.food
        );


        // 1. Constrói Fazenda de Comida em (6, 2) e planta Fruta Silvestre
        let farm_id = self.world.build_farm(6, 2, 50).expect("Farm building should succeed");
        let berry = self.crops_catalog.get("wild_berry").expect("Wild berry should exist");
        self.world.plant_crop(farm_id, berry).expect("Crop planting should succeed");

        // 2. Deposita ovo de Besta Vulcânica no ninho
        if let Some(volcano) = self.catalog.get("volcano_beast") {
            let egg_id = self.world.deposit_egg(volcano).expect("Egg deposit should succeed");
            println!(
                "Ninho: Ovo #{} de '{}' depositado.",
                egg_id, volcano.name
            );

            // Equipa modificador de velocidade no ninho
            self.world.hatchery.add_modifier(IncubationSpeedModifier::permanent(
                "primal_heat_amulet",
                "Amuleto do Calor Primordial",
                ModifierSource::Amulet("primal_heat_amulet".to_string()),
                2.5,
            ));
        }
    }

    fn fixed_update(&mut self, fixed_dt: f32) {
        self.ticks_processed += 1;

        // Avança simulação centralizada do mundo inteiro
        self.world.tick(
            fixed_dt as f64 * 30.0,
            &self.catalog,
            &self.habitats_catalog,
        );
    }

    fn update(&mut self, _dt: f32) {
        // Animações e apresentação
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

    // Executa passos do loop para simular fazendas e incubação
    for _ in 0..40 {
        app.step(1.0 / 60.0);
    }

    let state = app.state_mut();

    // 1. Colheita de comida na fazenda
    let farm_id = state.world.farms.keys().next().copied().unwrap();
    let food_harvested = state.world.harvest_farm(farm_id).expect("Harvest should succeed");
    println!("Colheita concluída: +{} de comida.", food_harvested);

    // 2. Alimenta a criatura inicial (Ignis)
    let ignis_id = state.world.creatures.keys().next().copied().unwrap();
    let feed_res = state.world.feed_creature(ignis_id).expect("Feeding should succeed");
    println!(
        "Criatura #{} alimentada. Nível: {}, Estágio: {:?}",
        ignis_id, feed_res.new_level, feed_res.new_stage
    );

    // 3. Eclosão do ovo da Besta Vulcânica e alocação no Habitat #10
    let starter_hab_id = state.world.habitats.keys().next().copied().unwrap();
    let newborn_id = state
        .world
        .hatch_egg(1, starter_hab_id, &state.catalog, &state.habitats_catalog)
        .expect("Egg hatching should succeed");

    println!(
        "ECLOSÃO! Criatura #{} nasceu e foi alocada no Habitat #{}.",
        newborn_id, starter_hab_id
    );

    // 4. Reivindica Marco do Bestiário
    let _reward = state
        .world
        .claim_milestone("first_discovery", &state.milestones_catalog, &state.catalog)
        .expect("Milestone claim should succeed");

    println!(
        "Bestiário: {}/{} espécies ({:.1}% concluído). Ouro total: {}, Comida total: {}",
        state.world.bestiary.discovered_count(),
        state.catalog.count(),
        state.world.bestiary.completion_percentage(&state.catalog),
        state.world.gold,
        state.world.food
    );

    // 5. Teste de Serialização de Savegame (JSON)
    let save_json = state.world.to_json().expect("World serialization should succeed");
    println!(
        "Savegame serializado com sucesso! Tamanho do save: {} bytes.",
        save_json.len()
    );

    // 6. Duelo na Arena Elemental
    println!("\n--- DUELO NA ARENA ELEMENTAL ---");
    let newborn = state.world.creatures.get(&newborn_id).unwrap();
    let volcano_spec = state.catalog.get(&newborn.species_id).unwrap();
    let player_fighter = Combatant::from_creature(newborn, volcano_spec);

    let enemy_spec = state.catalog.get("stone_golem").unwrap();
    let enemy_inst = game_core::CreatureInstance::new(99, &enemy_spec.id);
    let enemy_fighter = Combatant::from_creature(&enemy_inst, enemy_spec);

    let mut battle = BattleSession::new(vec![player_fighter], vec![enemy_fighter]);
    let (log1, _) = battle.execute_round(1, &state.elemental_chart);
    if let Some(l1) = log1 {
        println!(
            "  Ataque: {} usou '{}' contra {} (Mult: {:.1}x, Dano: {}, HP restante: {})",
            l1.attacker_name, l1.skill_name, l1.target_name, l1.elemental_multiplier, l1.raw_damage, l1.target_remaining_hp
        );
    }
    println!("Resultado da Arena: {:?}", battle.outcome);
}
