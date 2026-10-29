//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{
    App, BitmapFont, Camera2D, Color, EngineConfig, GameState, InputManager,
    IsometricProjection, MouseButton, Rect, RenderQueue, SpriteBatch,
    TextureAtlas, TouchPhase, UiAction, UiButton, UiColor, UiManager, UiModal, UiProgressBar,
};
use game_core::{
    BattleSession, Combatant, CropCatalog, ElementRegistry, ElementalChart, HabitatCatalog,
    IncubationSpeedModifier, MilestoneCatalog, ModifierSource, SanctuaryWorld, SpeciesCatalog,
};

pub struct BestigardGame {
    pub elements: ElementRegistry,
    pub catalog: SpeciesCatalog,
    pub habitats_catalog: HabitatCatalog,
    pub crops_catalog: CropCatalog,
    pub milestones_catalog: MilestoneCatalog,
    pub elemental_chart: ElementalChart,
    pub world: SanctuaryWorld,
    pub atlas: TextureAtlas,
    pub batch: SpriteBatch,
    pub camera: Camera2D,
    pub projection: IsometricProjection,
    pub render_queue: RenderQueue,
    pub font: BitmapFont,
    pub ui: UiManager,
    pub selected_creature_id: Option<u64>,
    pub selected_habitat_id: Option<u64>,
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

        let mut ui = UiManager::new(1080.0, 1920.0);
        // Botões rápidos do HUD Superior
        ui.add_global_button(
            UiButton::new("btn_collect_all", Rect::new(550.0, 24.0, 150.0, 56.0), "COLETAR")
                .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                .with_scale(1.8),
        );
        ui.add_global_button(
            UiButton::new("btn_open_creature", Rect::new(715.0, 24.0, 160.0, 56.0), "CRIATURA")
                .with_colors(UiColor::BTN_ACCENT, Color::WHITE)
                .with_scale(1.8),
        );
        ui.add_global_button(
            UiButton::new("btn_open_breeding", Rect::new(890.0, 24.0, 150.0, 56.0), "CRUZAR")
                .with_colors(UiColor::GOLD_ACCENT, Color::rgb(0.08, 0.12, 0.22))
                .with_scale(1.8),
        );

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
            ui,
            selected_creature_id: None,
            selected_habitat_id: None,
            frames_rendered: 0,
            ticks_processed: 0,
        }
    }

    /// Abre janela modal de inspeção e gestão de criatura (alimentação, nível e evolução).
    pub fn open_creature_modal(&mut self, creature_id: u64) {
        self.selected_creature_id = Some(creature_id);
        let creature = match self.world.creatures.get(&creature_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let species = match self.catalog.get(&creature.species_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let mut modal = UiModal::new(
            "modal_creature",
            "GESTAO DE CRIATURA",
            Rect::new(140.0, 280.0, 800.0, 680.0),
        );

        let nick = creature.nickname.as_deref().unwrap_or(&species.name);
        modal.add_text(
            format!("ESPECIE: {} ('{}')", species.name.to_uppercase(), nick.to_uppercase()),
            170.0,
            350.0,
            1.8,
            Color::WHITE,
        );

        let el_str = species
            .elements
            .iter()
            .map(|e| e.as_str().to_uppercase())
            .collect::<Vec<_>>()
            .join(" / ");
        modal.add_text(
            format!("ELEMENTOS: [{}] | RARIDADE: {:?}", el_str, species.rarity),
            170.0,
            390.0,
            1.6,
            Color::rgb(0.95, 0.82, 0.35),
        );

        modal.add_text(
            format!("ESTAGIO: {:?} | NIVEL ATUAL: {}", creature.stage, creature.level),
            170.0,
            430.0,
            1.8,
            Color::rgb(0.35, 0.90, 0.90),
        );

        modal.add_text(
            format!(
                "XP ACUMULADO: {} / {} PARA PROXIMO NIVEL",
                creature.xp,
                creature.xp_for_next_level()
            ),
            170.0,
            470.0,
            1.5,
            Color::WHITE,
        );

        // Barra de progresso de experiência (XP)
        let ratio = (creature.xp as f32 / creature.xp_for_next_level().max(1) as f32).clamp(0.0, 1.0);
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(170.0, 500.0, 740.0, 24.0),
            ratio,
            UiColor::XP_FILL,
        ));

        let gold_hr = creature.calculate_gold_per_hour(&species);
        modal.add_text(
            format!("PRODUCAO PASSIVA: {} OURO/HORA", gold_hr),
            170.0,
            545.0,
            1.6,
            Color::rgb(1.0, 0.85, 0.2),
        );

        modal.add_text(
            format!("COMIDA EM ESTOQUE NO SANTUARIO: {}", self.world.food),
            170.0,
            580.0,
            1.6,
            Color::rgb(0.4, 0.9, 0.3),
        );

        // Custo dinâmico de alimentação: 20 * level
        let feed_cost = 20 * creature.level as u64;
        let can_feed = self.world.food >= feed_cost;

        let feed_btn = UiButton::new(
            "btn_feed_action",
            Rect::new(190.0, 640.0, 700.0, 68.0),
            format!("ALIMENTAR CRIATURA (-{} COMIDA)", feed_cost),
        )
        .with_colors(
            if can_feed { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
            Color::WHITE,
        )
        .with_disabled(!can_feed)
        .with_scale(1.9);

        modal.add_button(feed_btn);

        // Dica de evolução
        let evo_msg = if creature.level < 4 {
            "Evolui para Jovem no Nivel 4 (Pronto para Cruzamento)"
        } else if creature.level < 10 {
            "Evolui para Adulto Titanico no Nivel 10 (Pico de Poder)"
        } else {
            "Estagio Maximo de Adulto Atingido!"
        };
        modal.add_text(evo_msg, 190.0, 730.0, 1.5, UiColor::GOLD_ACCENT);

        self.ui.open_modal(modal);
    }

    /// Abre janela modal para seleção de progenitores e início do cruzamento determinístico.
    pub fn open_breeding_modal(&mut self) {
        let mut modal = UiModal::new(
            "modal_breeding",
            "ALTAR DE CRUZAMENTO GENETICO",
            Rect::new(140.0, 280.0, 800.0, 680.0),
        );

        // Identifica criaturas adultas/jovens elegíveis para cruzar (nível >= 4)
        let eligible: Vec<u64> = self
            .world
            .creatures
            .iter()
            .filter(|(_, c)| c.can_breed())
            .map(|(&id, _)| id)
            .collect();

        modal.add_text(
            "SELECAO DE PAIS PARA COMBINACAO DE ELEMENTOS",
            170.0,
            350.0,
            1.8,
            Color::WHITE,
        );

        if eligible.len() >= 2 {
            let p1 = &self.world.creatures[&eligible[0]];
            let p2 = &self.world.creatures[&eligible[1]];
            let s1 = self.catalog.get(&p1.species_id).unwrap();
            let s2 = self.catalog.get(&p2.species_id).unwrap();

            modal.add_text(
                format!("PAI 1: {} (NIVEL {})", s1.name.to_uppercase(), p1.level),
                170.0,
                400.0,
                1.7,
                Color::rgb(1.0, 0.65, 0.35),
            );
            modal.add_text(
                format!("PAI 2: {} (NIVEL {})", s2.name.to_uppercase(), p2.level),
                170.0,
                440.0,
                1.7,
                Color::rgb(0.55, 0.85, 1.0),
            );

            modal.add_text(
                "PREVISAO GENETICA: HIBRIDO ELEMENTAL COMPATIVEL",
                170.0,
                490.0,
                1.6,
                UiColor::GOLD_ACCENT,
            );
            modal.add_text(
                "Chance aumentada de mutacao rara pelo nivel dos progenitores!",
                170.0,
                525.0,
                1.5,
                Color::WHITE,
            );

            let breed_btn = UiButton::new(
                "btn_breed_action",
                Rect::new(190.0, 620.0, 700.0, 68.0),
                "INICIAR CRUZAMENTO (GERAR OVO NO NINHO)",
            )
            .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
            .with_scale(1.8);
            modal.add_button(breed_btn);
        } else {
            modal.add_text(
                "CRIATURAS ADULTAS INSUFICIENTES (MINIMO: 2 NIVEL 4+)",
                170.0,
                410.0,
                1.6,
                UiColor::BTN_DANGER,
            );
            modal.add_text(
                "Dica: Alimente suas criaturas ate o Nivel 4 (Juvenil) para liberá-las!",
                170.0,
                460.0,
                1.5,
                Color::WHITE,
            );
            modal.add_text(
                format!(
                    "Criaturas prontas: {} / 2 requeridas.",
                    eligible.len()
                ),
                170.0,
                500.0,
                1.6,
                UiColor::GOLD_ACCENT,
            );
        }

        self.ui.open_modal(modal);
    }

    /// Abre janela modal de inspeção e coleta de ouro do habitat selecionado.
    pub fn open_habitat_modal(&mut self, habitat_id: u64) {
        self.selected_habitat_id = Some(habitat_id);
        let hab = match self.world.habitats.get(&habitat_id) {
            Some(h) => h.clone(),
            None => return,
        };
        let hab_type = match self.habitats_catalog.get(&hab.type_id) {
            Some(t) => t.clone(),
            None => return,
        };
        let tier_info = hab_type.get_tier_info(hab.tier);
        let gold_cap = tier_info.map(|t| t.gold_capacity).unwrap_or(500);
        let max_creatures = tier_info.map(|t| t.max_creatures).unwrap_or(2);

        let mut modal = UiModal::new(
            "modal_habitat",
            "GESTAO DE HABITAT",
            Rect::new(140.0, 280.0, 800.0, 600.0),
        );

        modal.add_text(
            format!("TIPO: {} (TIER {})", hab_type.name.to_uppercase(), hab.tier),
            170.0,
            350.0,
            1.8,
            Color::WHITE,
        );
        let elements_str = hab_type
            .supported_elements
            .iter()
            .map(|e| e.as_str().to_uppercase())
            .collect::<Vec<_>>()
            .join(" / ");
        modal.add_text(
            format!("ELEMENTOS: [{}]", elements_str),
            170.0,
            390.0,
            1.6,
            Color::rgb(1.0, 0.75, 0.35),
        );
        modal.add_text(
            format!(
                "OCUPACAO: {} / {} CRIATURAS",
                hab.creature_ids.len(),
                max_creatures
            ),
            170.0,
            430.0,
            1.6,
            Color::WHITE,
        );
        modal.add_text(
            format!(
                "OURO ACUMULADO: {} / {} LIMITE",
                hab.accumulated_gold, gold_cap
            ),
            170.0,
            470.0,
            1.7,
            Color::rgb(1.0, 0.85, 0.2),
        );

        // Barra de acúmulo de ouro
        let ratio = (hab.accumulated_gold as f32 / gold_cap as f32).clamp(0.0, 1.0);
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(170.0, 500.0, 740.0, 24.0),
            ratio,
            UiColor::GOLD_FILL,
        ));

        let has_gold = hab.accumulated_gold > 0;
        let collect_btn = UiButton::new(
            "btn_collect_hab_action",
            Rect::new(190.0, 560.0, 700.0, 68.0),
            format!("COLETAR +{} DE OURO", hab.accumulated_gold),
        )
        .with_colors(
            if has_gold { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
            Color::WHITE,
        )
        .with_disabled(!has_gold)
        .with_scale(1.9);
        modal.add_button(collect_btn);

        self.ui.open_modal(modal);
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

        // 4. Camada de Interface de Usuário (Ui): HUD Superior fixo de recursos
        // Barra de vidro fosco do HUD
        self.render_queue.push_ui_rect(
            Rect::new(20.0, 16.0, 1040.0, 72.0),
            UiColor::DARK_GLASS,
        );
        // Friso decorativo inferior dourado
        self.render_queue.push_ui_rect(
            Rect::new(20.0, 88.0, 1040.0, 2.0),
            UiColor::GOLD_ACCENT,
        );

        // Ícones de recursos no HUD
        if let Some(gold) = self.atlas.get("icon_gold_coin") {
            self.render_queue.push_ui(gold.clone(), Rect::new(32.0, 28.0, 48.0, 48.0), Color::WHITE);
        }
        if let Some(food) = self.atlas.get("icon_food_apple") {
            self.render_queue.push_ui(food.clone(), Rect::new(280.0, 28.0, 48.0, 48.0), Color::WHITE);
        }

        // Flush com ordenação de profundidade automática (Y-Sort) diretamente no SpriteBatch
        self.render_queue.flush_to_batch(&mut self.batch);

        // 5. Renderização dos Textos do HUD Superior com BitmapFont
        self.font.draw_text(
            &mut self.batch,
            &format!("OURO: {}", self.world.gold),
            88.0,
            42.0,
            2.0,
            UiColor::GOLD_FILL,
        );
        self.font.draw_text(
            &mut self.batch,
            &format!("COMIDA: {}", self.world.food),
            336.0,
            42.0,
            2.0,
            Color::rgb(0.4, 0.9, 0.3),
        );
        self.font.draw_text(
            &mut self.batch,
            "BESTIGARD SANCTUARY",
            40.0,
            104.0,
            1.8,
            Color::WHITE,
        );

        // 6. Renderização da Interface e Modais via UiManager
        self.ui.render(&mut self.render_queue, &mut self.batch, &self.font);
    }

    fn on_resume(&mut self) {
        println!("Bestigard retomado (foreground mobile)");
    }

    fn on_input(&mut self, input: &InputManager) {
        let mut handled_tap = false;

        for tap in input.taps() {
            let action = self.ui.handle_tap(tap);
            match action {
                UiAction::ButtonClicked(ref id) => {
                    handled_tap = true;
                    match id.as_str() {
                        "btn_collect_all" => {
                            let total = self.world.collect_all_gold();
                            println!("UI: Coletado ouro de todos os habitats: +{} ouro!", total);
                        }
                        "btn_open_creature" => {
                            if let Some(&first_id) = self.world.creatures.keys().next() {
                                self.open_creature_modal(first_id);
                            }
                        }
                        "btn_open_breeding" => {
                            self.open_breeding_modal();
                        }
                        "btn_feed_action" => {
                            if let Some(cid) = self.selected_creature_id {
                                match self.world.feed_creature(cid) {
                                    Ok(res) => {
                                        println!(
                                            "UI: Criatura #{} alimentada! Novo nivel: {}, Estagio: {:?}",
                                            cid, res.new_level, res.new_stage
                                        );
                                        // Atualiza o modal com as informações renovadas
                                        self.open_creature_modal(cid);
                                    }
                                    Err(e) => println!("UI: Falha ao alimentar criatura: {}", e),
                                }
                            }
                        }
                        "btn_breed_action" => {
                            let eligible: Vec<u64> = self
                                .world
                                .creatures
                                .iter()
                                .filter(|(_, c)| c.can_breed())
                                .map(|(&id, _)| id)
                                .collect();
                            if eligible.len() >= 2 {
                                match self.world.breed_creatures(eligible[0], eligible[1], &self.catalog, 9999) {
                                    Ok(res) => {
                                        println!(
                                            "UI: Cruzamento bem sucedido! Ovo da especie '{}' depositado no ninho!",
                                            res.offspring_species_id
                                        );
                                        self.ui.close_modal();
                                    }
                                    Err(e) => println!("UI: Erro no cruzamento: {}", e),
                                }
                            }
                        }
                        "btn_collect_hab_action" => {
                            if let Some(hid) = self.selected_habitat_id {
                                match self.world.collect_habitat_gold(hid) {
                                    Ok(gold) => {
                                        println!("UI: Coletado +{} ouro do Habitat #{}!", gold, hid);
                                        self.open_habitat_modal(hid);
                                    }
                                    Err(e) => println!("UI: Erro na coleta do habitat: {}", e),
                                }
                            }
                        }
                        _ => {}
                    }
                }
                UiAction::ModalClosed => {
                    handled_tap = true;
                    println!("UI: Modal fechado pelo usuário.");
                }
                UiAction::None => {}
            }

            if !handled_tap && !self.ui.is_modal_open() {
                let cell = self.camera.screen_to_grid_cell(tap.x, tap.y, &self.projection);
                println!(
                    "Entrada Unificada: Tap capturado em ({:.0}, {:.0}) -> Célula da ilha selecionada: ({}, {})",
                    tap.x, tap.y, cell.0, cell.1
                );

                // Se tocou sobre o habitat (células 2..5 x 2..5), abre modal do habitat
                if let Some((&hab_id, _)) = self.world.habitats.iter().find(|(_, h)| {
                    let hx = h.grid_x as i32;
                    let hy = h.grid_y as i32;
                    cell.0 >= hx && cell.0 < hx + 3 && cell.1 >= hy && cell.1 < hy + 3
                }) {
                    println!("UI: Selecionado Habitat #{} no mapa -> Abrindo menu de gestão", hab_id);
                    self.open_habitat_modal(hab_id);
                }
            }
        }

        // Se nenhum modal estiver aberto, gestos de pan e pinch movem a câmera normalmente
        if !self.ui.is_modal_open() {
            input.apply_to_camera(&mut self.camera);
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

    // 7. Demonstração de Entradas Unificadas Mobile/Desktop (Tap, Drag, Pinch-to-Zoom)
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

    // 8. Demonstração Interativa da Interface de Usuário (HUD e Menus Modais)
    println!("\n--- INTERFACE DE USUÁRIO (HUD & MENUS MODAIS) ---");

    // A) Toque no botão global do HUD: [COLETAR]
    println!("1. Testando toque no botão de HUD [COLETAR OURO] (pos: 600, 40)...");
    let mut hud_input = InputManager::new();
    hud_input.on_mouse_button_down(MouseButton::Left, 600.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 600.0, 40.0);
    state.on_input(&hud_input);
    hud_input.clear_events();

    // B) Toque no botão global do HUD: [CRIATURA] para abrir modal de inspeção
    println!("\n2. Abrindo diálogo modal de Gestão da Criatura Ignis [CRIATURA] (pos: 750, 40)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 750.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 750.0, 40.0);
    state.on_input(&hud_input);
    hud_input.clear_events();
    assert!(state.ui.is_modal_open());
    println!("Modal ativo: '{}'", state.ui.active_modal.as_ref().unwrap().title);

    // C) Toque no botão interno do modal: [ALIMENTAR CRIATURA]
    println!("\n3. Alimentando criatura através do botão interno do modal (pos: 400, 670)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 400.0, 670.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 400.0, 670.0);
    state.on_input(&hud_input);
    hud_input.clear_events();
    assert!(state.ui.is_modal_open());

    // D) Fechar modal pelo botão de fechar (X)
    let close_x = state.ui.active_modal.as_ref().unwrap().close_button.rect.x + 10.0;
    let close_y = state.ui.active_modal.as_ref().unwrap().close_button.rect.y + 10.0;
    println!("\n4. Fechando modal via botão de fechar (X) (pos: {:.0}, {:.0})...", close_x, close_y);
    hud_input.on_mouse_button_down(MouseButton::Left, close_x, close_y);
    hud_input.on_mouse_button_up(MouseButton::Left, close_x, close_y);
    state.on_input(&hud_input);
    hud_input.clear_events();
    assert!(!state.ui.is_modal_open());

    // E) Promover criaturas a Nível 4 e testar Altar de Cruzamento
    println!("\n5. Promovendo Ignis e Besta Vulcânica para Nível 4 (Juvenil) para habilitar cruzamento...");
    for creature in state.world.creatures.values_mut() {
        creature.level = 4;
        creature.stage = game_core::GrowthStage::from_level(4);
    }

    println!("Abrindo modal do [CRUZAMENTO] (pos: 920, 40)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 920.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 920.0, 40.0);
    state.on_input(&hud_input);
    hud_input.clear_events();
    assert!(state.ui.is_modal_open());
    println!("Modal ativo: '{}'", state.ui.active_modal.as_ref().unwrap().title);

    println!("\n6. Executando cruzamento pelo botão [INICIAR CRUZAMENTO] (pos: 400, 650)...");
    let initial_eggs = state.world.hatchery.eggs.len();
    hud_input.on_mouse_button_down(MouseButton::Left, 400.0, 650.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 400.0, 650.0);
    state.on_input(&hud_input);
    hud_input.clear_events();
    assert_eq!(state.world.hatchery.eggs.len(), initial_eggs + 1);
    println!("Ninho de Incubação: {} ovo(s) em processo de choca!", state.world.hatchery.eggs.len());

    // F) Simular geração de ouro e abrir modal do Habitat #10 para coletar
    println!("\n7. Simulando acumulo de ouro e abrindo modal do Habitat #{}...", starter_hab_id);
    state.world.habitats.get_mut(&starter_hab_id).unwrap().accumulated_gold = 150;
    state.open_habitat_modal(starter_hab_id);
    assert!(state.ui.is_modal_open());

    println!("Coletando ouro acumulado pelo botão do modal (pos: 400, 590)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 400.0, 590.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 400.0, 590.0);
    state.on_input(&hud_input);
    hud_input.clear_events();

    state.ui.close_modal();
    assert!(!state.ui.is_modal_open());

    println!("\n>> Interface de Usuário (HUD e Menus Modais) validada com 100% de sucesso! <<");
}
