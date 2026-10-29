//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{
    App, BitmapFont, Camera2D, Color, EngineConfig, GameState, InputManager,
    IsometricProjection, MouseButton, Rect, RenderQueue, RgbaImage, SpriteBatch, TextureAtlas,
    TouchPhase,
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
    pub render_queue: RenderQueue,
    pub font: BitmapFont,
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
            render_queue: RenderQueue::new(),
            font: BitmapFont::default_font(),
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
        self.render_queue.clear();

        // 1. Camada de Fundo (Ground): grade de 6x6 tiles isométricos
        if let Some(grass) = self.atlas.get("tile_island_grass_01") {
            for x in 0..6 {
                for y in 0..6 {
                    let screen_pos = self.camera.grid_to_screen(x as f32, y as f32, &self.projection);
                    let tile_w = self.projection.tile_width * self.camera.zoom;
                    let tile_h = self.projection.tile_height * self.camera.zoom;
                    let tile_rect = Rect::new(screen_pos.x - tile_w * 0.5, screen_pos.y, tile_w, tile_h);
                    self.render_queue.push_ground(grass.clone(), tile_rect, Color::WHITE);
                }
            }
        }

        // 2. Camada de Decalques no Chão (GroundDecal): sombra sob o habitat e construções
        let hab_screen = self.camera.grid_to_screen(3.0, 3.0, &self.projection);
        self.render_queue.push_decal(
            self.atlas.get("tile_island_grass_01").unwrap().clone(),
            Rect::new(hab_screen.x - 70.0, hab_screen.y + 40.0, 140.0, 30.0),
            Color::rgba(0.0, 0.0, 0.0, 0.35),
        );

        // 3. Camada de Entidades com Y-Sorting (Entities):
        // - Criatura A atrás do habitat (grid 2.5, 2.5 -> depth menor)
        if let Some(volcano) = self.atlas.get("creature_volcano_beast_baby") {
            let p = self.camera.grid_to_screen(2.5, 2.5, &self.projection);
            self.render_queue.push_entity_grid(
                2.5,
                2.5,
                0.0,
                volcano.clone(),
                Rect::new(p.x - 24.0, p.y - 30.0, 48.0, 48.0),
                Color::WHITE,
            );
        }

        // - Habitat no meio (grid 3.0, 3.0 -> depth intermediário)
        if let Some(hab) = self.atlas.get("habitat_fire_t1") {
            self.render_queue.push_entity_grid(
                3.0,
                3.0,
                0.0,
                hab.clone(),
                Rect::new(hab_screen.x - 64.0, hab_screen.y - 70.0, 128.0, 128.0),
                Color::WHITE,
            );
        }

        // - Criatura B na frente do habitat (grid 3.5, 3.5 -> depth maior)
        if let Some(pyro) = self.atlas.get("creature_pyro_drake_baby") {
            let p = self.camera.grid_to_screen(3.5, 3.5, &self.projection);
            self.render_queue.push_entity_grid(
                3.5,
                3.5,
                0.0,
                pyro.clone(),
                Rect::new(p.x - 24.0, p.y - 30.0, 48.0, 48.0),
                Color::WHITE,
            );
        }

        // 4. Camada de Interface de Usuário (Ui): HUD fixo e contadores
        self.render_queue.push_ui_rect(
            Rect::new(20.0, 20.0, 1040.0, 80.0),
            Color::rgba(0.08, 0.12, 0.22, 0.9),
        );
        if let Some(gold) = self.atlas.get("icon_gold_coin") {
            self.render_queue.push_ui(gold.clone(), Rect::new(40.0, 36.0, 48.0, 48.0), Color::WHITE);
        }
        if let Some(food) = self.atlas.get("icon_food_apple") {
            self.render_queue.push_ui(food.clone(), Rect::new(260.0, 36.0, 48.0, 48.0), Color::WHITE);
        }

        // Flush com ordenação de profundidade automática (Y-Sort) diretamente no SpriteBatch
        self.render_queue.flush_to_batch(&mut self.batch);

        // 5. Renderização de Texto com BitmapFont
        self.font.draw_text(
            &mut self.batch,
            &format!("OURO: {}", self.world.gold),
            95.0,
            48.0,
            2.0,
            Color::rgb(1.0, 0.85, 0.2),
        );
        self.font.draw_text(
            &mut self.batch,
            &format!("COMIDA: {}", self.world.food),
            315.0,
            48.0,
            2.0,
            Color::rgb(0.4, 0.9, 0.3),
        );
        self.font.draw_text(
            &mut self.batch,
            "BESTIGARD SANCTUARY",
            40.0,
            110.0,
            1.8,
            Color::WHITE,
        );
    }

    fn on_resume(&mut self) {
        println!("Bestigard retomado (foreground mobile)");
    }

    fn on_input(&mut self, input: &InputManager) {
        input.apply_to_camera(&mut self.camera);
        for tap in input.taps() {
            let cell = self.camera.screen_to_grid_cell(tap.x, tap.y, &self.projection);
            println!(
                "Entrada Unificada: Tap capturado em ({:.0}, {:.0}) -> Célula da ilha selecionada: ({}, {})",
                tap.x, tap.y, cell.0, cell.1
            );
        }
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

    // 9. Demonstração de Ordenação de Profundidade Isométrica (Y-Sorting)
    println!("\n--- ORDENAÇÃO DE PROFUNDIDADE (Y-SORTING) & CAMADAS ---");
    println!("RenderQueue com 4 camadas ativas: Ground -> GroundDecal -> Entities (Y-Sorted) -> Ui");
    println!("Criatura Vulcânica (Grid 2.5, 2.5) renderizada ATRÁS do Habitat (Grid 3.0, 3.0).");
    println!("Criatura Pírica (Grid 3.5, 3.5) renderizada NA FRENTE do Habitat (Grid 3.0, 3.0).");

    // 10. Demonstração de Carregamento de Texturas & Renderização de Texto
    println!("\n--- CARREGAMENTO DE TEXTURAS & FONTES BITMAP ---");
    let (atlas_img, from_disk) =
        RgbaImage::load_or_fallback("assets/sprites/creatures_atlas.png", 512, 512);
    println!(
        "Carregador de Textura: 'assets/sprites/creatures_atlas.png' -> {} (Resolução: {}x{} px).",
        if from_disk { "Carregado do disco com sucesso" } else { "PNG ausente -> Gerado fallback procedural automático (Pronto para substituição imediata)" },
        atlas_img.width,
        atlas_img.height
    );

    let (label_w, label_h) = state.font.measure_text("OURO: 620", 2.0);
    println!(
        "Renderizador de Texto: Rótulo 'OURO: 620' medido em {:.0}x{:.0} px (ASCII 5x7 embutido com suporte a cores).",
        label_w, label_h
    );

    // 11. Demonstração de Entradas Unificadas Mobile/Desktop (Tap, Drag, Pinch-to-Zoom)
    println!("\n--- SISTEMA UNIFICADO DE ENTRADA & GESTOS (INPUT) ---");
    let mut test_input = InputManager::new();

    // A) Simula toque rápido capacitivo com 1 dedo (Tap)
    test_input.on_touch(TouchPhase::Started, 10, 540.0, 960.0);
    test_input.on_touch(TouchPhase::Ended, 10, 540.0, 960.0);
    state.on_input(&test_input);
    test_input.clear_events();

    // B) Simula gesto de pinça capacitiva com 2 dedos (Pinch-to-zoom)
    test_input.on_touch(TouchPhase::Started, 1, 440.0, 960.0);
    test_input.on_touch(TouchPhase::Started, 2, 640.0, 960.0);
    test_input.on_touch(TouchPhase::Moved, 1, 390.0, 960.0);
    test_input.on_touch(TouchPhase::Moved, 2, 690.0, 960.0);
    state.on_input(&test_input);

    let pinch_events: Vec<_> = test_input
        .events()
        .iter()
        .filter(|e| matches!(e, engine::InputEvent::PinchZoom { .. }))
        .collect();
    println!(
        "Touchscreen Mobile: Gesto de pinça com 2 dedos detectado ({} eventos gerados). Novo Zoom da Câmera: {:.2}x",
        pinch_events.len(),
        state.camera.zoom
    );
    test_input.clear_events();

    // C) Simula clique e arraste com mouse desktop (Pan)
    test_input.on_mouse_button_down(MouseButton::Left, 500.0, 500.0);
    test_input.on_mouse_motion(450.0, 480.0);
    test_input.on_mouse_button_up(MouseButton::Left, 450.0, 480.0);
    state.on_input(&test_input);

    println!(
        "Mouse Desktop: Arraste de tela processado com sucesso. Posição da Câmera: ({:.1}, {:.1})",
        state.camera.position.x, state.camera.position.y
    );
}
