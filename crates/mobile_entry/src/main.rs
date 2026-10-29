//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{
    App, Camera2D, Color, EngineConfig, GameState, IsometricProjection, Rect, SpriteBatch,
    TextureAtlas,
};
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
    pub atlas: TextureAtlas,
    pub batch: SpriteBatch,
    pub camera: Camera2D,
    pub projection: IsometricProjection,
    frames_rendered: u64,
    ticks_processed: u64,
}

impl BestigardGame {
    pub fn new() -> Self {
        let catalog = SpeciesCatalog::with_starter_species();
        let habitats_catalog = HabitatCatalog::with_default_habitats();

        let world = SanctuaryWorld::with_starter_setup("Mestre das Bestas", &catalog, &habitats_catalog)
            .expect("Starter world setup should succeed");

        let mut atlas = TextureAtlas::new("sanctuary_atlas", 2048, 2048);
        atlas.add_region("tile_island_grass_01", Rect::new(0.0, 0.0, 128.0, 64.0));
        atlas.add_region("habitat_fire_t1", Rect::new(128.0, 0.0, 192.0, 192.0));
        atlas.add_region("building_farm_empty", Rect::new(320.0, 0.0, 160.0, 160.0));
        atlas.add_region("creature_pyro_drake_baby", Rect::new(480.0, 0.0, 64.0, 64.0));
        atlas.add_region("creature_volcano_beast_baby", Rect::new(544.0, 0.0, 64.0, 64.0));
        atlas.add_region("icon_gold_coin", Rect::new(608.0, 0.0, 32.0, 32.0));
        atlas.add_region("icon_food_apple", Rect::new(640.0, 0.0, 32.0, 32.0));

        let mut camera = Camera2D::new(1080.0, 1920.0);
        let projection = IsometricProjection::DEFAULT;
        // Centraliza a câmera no meio do terreno inicial do santuário
        let center_world = projection.grid_to_world(5.0, 5.0);
        camera.set_position(center_world);

        Self {
            elements: ElementRegistry::with_default_elements(),
            catalog,
            habitats_catalog,
            crops_catalog: CropCatalog::with_default_crops(),
            milestones_catalog: MilestoneCatalog::with_default_milestones(),
            elemental_chart: ElementalChart::with_default_affinities(),
            world,
            atlas,
            batch: SpriteBatch::new(1024),
            camera,
            projection,
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
        self.batch.clear();

        // 1. Batcheia fundo do santuário
        self.batch.draw_colored_rect(Rect::new(0.0, 0.0, 1080.0, 1920.0), Color::AETHER_NAVY);

        // 2. Batcheia grade de tiles de terreno com projeção isométrica 2:1 e câmera
        if let Some(grass) = self.atlas.get("tile_island_grass_01") {
            for x in 0..6 {
                for y in 0..6 {
                    let screen_pos = self.camera.grid_to_screen(x as f32, y as f32, &self.projection);
                    let tile_w = self.projection.tile_width * self.camera.zoom;
                    let tile_h = self.projection.tile_height * self.camera.zoom;
                    let tile_rect = Rect::new(
                        screen_pos.x - tile_w * 0.5,
                        screen_pos.y,
                        tile_w,
                        tile_h,
                    );
                    self.batch.draw_sprite(grass, tile_rect, Color::WHITE);
                }
            }
        }

        // 3. Batcheia habitats e fazendas
        if let Some(hab) = self.atlas.get("habitat_fire_t1") {
            self.batch.draw_sprite(hab, Rect::new(320.0, 460.0, 128.0, 128.0), Color::WHITE);
        }
        if let Some(farm) = self.atlas.get("building_farm_empty") {
            self.batch.draw_sprite(farm, Rect::new(460.0, 460.0, 100.0, 100.0), Color::WHITE);
        }

        // 4. Batcheia criaturas no santuário
        if let Some(pyro) = self.atlas.get("creature_pyro_drake_baby") {
            self.batch.draw_sprite(pyro, Rect::new(360.0, 500.0, 48.0, 48.0), Color::WHITE);
        }
        if let Some(volcano) = self.atlas.get("creature_volcano_beast_baby") {
            self.batch.draw_sprite(volcano, Rect::new(400.0, 510.0, 48.0, 48.0), Color::WHITE);
        }

        // 5. Batcheia HUD e ícones de recursos
        self.batch.draw_colored_rect(Rect::new(20.0, 20.0, 1040.0, 80.0), Color::rgba(0.08, 0.12, 0.22, 0.9));
        if let Some(gold) = self.atlas.get("icon_gold_coin") {
            self.batch.draw_sprite(gold, Rect::new(40.0, 36.0, 48.0, 48.0), Color::WHITE);
        }
        if let Some(food) = self.atlas.get("icon_food_apple") {
            self.batch.draw_sprite(food, Rect::new(260.0, 36.0, 48.0, 48.0), Color::WHITE);
        }
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

    // 7. Demonstração do Pipeline de Renderização em Lote (Sprite Batcher)
    println!("\n--- PIPELINE 2D SPRITE BATCHER & ATLAS ---");
    let batch_stats = state.batch.stats();
    println!(
        "Atlas '{}': {} sprites mapeados (Resolução: {}x{} px).",
        state.atlas.name,
        state.atlas.len(),
        state.atlas.width,
        state.atlas.height
    );
    println!(
        "Lote consolidado: {} quads, {} vértices e {} índices agrupados para renderização.",
        batch_stats.quads,
        batch_stats.vertices,
        batch_stats.indices
    );

    // 8. Demonstração de Controles de Câmera e Raycasting Isométrico
    println!("\n--- CÂMERA 2D & PROJEÇÃO ISOMÉTRICA ---");
    let touch_screen = (540.0, 960.0); // Toque no centro da tela mobile
    let world_picked = state.camera.screen_to_world(touch_screen.0, touch_screen.1);
    let cell_picked = state.camera.screen_to_grid_cell(
        touch_screen.0,
        touch_screen.1,
        &state.projection,
    );

    println!(
        "Toque na tela em ({:.0}, {:.0}) -> Mundo: ({:.1}, {:.1}) -> Célula da Ilha: ({}, {})",
        touch_screen.0, touch_screen.1, world_picked.x, world_picked.y, cell_picked.0, cell_picked.1
    );

    // Simulação de gesto de pinça (zoom in 1.35x centrado no toque) e arrasto (pan)
    state.camera.zoom_at(1.35, touch_screen.0, touch_screen.1);
    state.camera.pan(-80.0, -40.0);

    println!(
        "Após Pan e Zoom: Câmera Pos: ({:.1}, {:.1}), Zoom: {:.2}x, Limites Visíveis: {:?}",
        state.camera.position.x,
        state.camera.position.y,
        state.camera.zoom,
        state.camera.visible_world_bounds()
    );
}
