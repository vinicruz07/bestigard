//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{
    App, BitmapFont, Camera2D, Color, EngineConfig, GameState, InputManager,
    IsometricProjection, MouseButton, PlacementCursor, Rect, RenderCommand, RenderLayer,
    RenderQueue, SpriteBatch, TextureAtlas, TouchPhase, UiAction, UiButton, UiColor, UiManager,
    UiModal, UiProgressBar,
};
use game_core::{
    BattleOutcome, BattleSession, Combatant, CropCatalog, ElementRegistry, ElementalChart,
    HabitatCatalog, IncubationSpeedModifier, MilestoneCatalog, ModifierSource, ObstacleKind,
    SanctuaryWorld, SpeciesCatalog, FARM_BUILD_COST_GOLD,
};

/// Estrutura que o jogador está posicionando no modo de construção.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildItem {
    Habitat(String),
    Farm,
}

/// Sessão ativa do modo de construção: item escolhido, custo e cursor sobre o grid.
#[derive(Debug, Clone)]
pub struct PlacementSession {
    pub item: BuildItem,
    pub label: String,
    pub cost: u64,
    pub cursor: PlacementCursor,
}

/// Sessão ativa de combate na Arena Elemental: combatentes, registro e recompensas.
#[derive(Debug, Clone)]
pub struct ArenaSessionState {
    pub battle: BattleSession,
    pub player_creature_id: u64,
    pub last_action_log: Option<String>,
    pub reward_gold: u64,
    pub reward_xp: u64,
    pub rewards_claimed: bool,
}

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
    pub selected_obstacle_id: Option<u64>,
    pub placement: Option<PlacementSession>,
    pub arena: Option<ArenaSessionState>,
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

        let ui = UiManager::new(1080.0, 1920.0);

        let mut game = Self {
            elements: ElementRegistry::with_default_elements(),
            catalog,
            habitats_catalog,
            crops_catalog: CropCatalog::with_default_crops(),
            milestones_catalog: MilestoneCatalog::with_default_milestones(),
            elemental_chart: ElementalChart::with_default_affinities(),
            world,
            atlas,
            batch: SpriteBatch::new(4096),
            camera,
            projection,
            render_queue: RenderQueue::new(),
            font: BitmapFont::default_font(),
            ui,
            selected_creature_id: None,
            selected_habitat_id: None,
            selected_obstacle_id: None,
            placement: None,
            arena: None,
            frames_rendered: 0,
            ticks_processed: 0,
        };
        game.configure_hud_buttons();
        game
    }

    /// Botões rápidos do HUD Superior (visíveis fora do modo de construção).
    fn configure_hud_buttons(&mut self) {
        self.ui.clear_global_buttons();
        self.ui.add_global_button(
            UiButton::new("btn_collect_all", Rect::new(450.0, 24.0, 112.0, 56.0), "COLETAR")
                .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                .with_scale(1.8),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_creature", Rect::new(570.0, 24.0, 112.0, 56.0), "CRIATURA")
                .with_colors(UiColor::BTN_ACCENT, Color::WHITE)
                .with_scale(1.8),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_breeding", Rect::new(690.0, 24.0, 112.0, 56.0), "CRUZAR")
                .with_colors(UiColor::GOLD_ACCENT, Color::rgb(0.08, 0.12, 0.22))
                .with_scale(1.8),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_shop", Rect::new(810.0, 24.0, 112.0, 56.0), "LOJA")
                .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                .with_scale(1.8),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_arena", Rect::new(930.0, 24.0, 115.0, 56.0), "ARENA")
                .with_colors(UiColor::BTN_DANGER, Color::WHITE)
                .with_scale(1.8),
        );
    }

    /// Botões flutuantes de confirmar/cancelar exibidos durante o modo de construção.
    fn configure_placement_buttons(&mut self) {
        self.ui.clear_global_buttons();
        self.ui.add_global_button(
            UiButton::new("btn_placement_confirm", Rect::new(150.0, 1740.0, 380.0, 96.0), "CONFIRMAR")
                .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                .with_scale(2.2),
        );
        self.ui.add_global_button(
            UiButton::new("btn_placement_cancel", Rect::new(550.0, 1740.0, 380.0, 96.0), "CANCELAR")
                .with_colors(UiColor::BTN_DANGER, Color::WHITE)
                .with_scale(2.2),
        );
    }

    /// Valida se o cursor de construção está sobre uma área livre da ilha.
    fn placement_is_valid(&self, cursor: &PlacementCursor) -> bool {
        match cursor.origin_unsigned() {
            Some((x, y)) => self
                .world
                .can_place_building(x, y, cursor.width, cursor.height)
                .is_ok(),
            None => false,
        }
    }

    /// Entra no modo de construção para o item escolhido, centralizando o cursor na tela.
    pub fn start_placement(&mut self, item: BuildItem) {
        let (label, cost, w, h) = match &item {
            BuildItem::Habitat(type_id) => match self.habitats_catalog.get(type_id) {
                Some(t) => (t.name.clone(), t.build_cost_gold, t.width, t.height),
                None => return,
            },
            BuildItem::Farm => ("Fazenda".to_string(), FARM_BUILD_COST_GOLD, 2, 2),
        };

        let mut cursor = PlacementCursor::new(w, h);
        let screen_center = self.camera.screen_to_grid_cell(540.0, 960.0, &self.projection);
        cursor.center_on_cell(screen_center);
        cursor.set_valid(self.placement_is_valid(&cursor));

        self.placement = Some(PlacementSession { item, label, cost, cursor });
        self.configure_placement_buttons();
    }

    /// Move o cursor de construção para a célula tocada e revalida a área.
    pub fn move_placement_to(&mut self, cell: (i32, i32)) {
        let Some(mut session) = self.placement.take() else { return };
        session.cursor.center_on_cell(cell);
        let valid = self.placement_is_valid(&session.cursor);
        session.cursor.set_valid(valid);
        self.placement = Some(session);
    }

    /// Sai do modo de construção sem comprar nada.
    pub fn cancel_placement(&mut self) {
        self.placement = None;
        self.configure_hud_buttons();
    }

    /// Confirma a compra: valida área e ouro, constrói a estrutura e sai do modo de construção.
    pub fn confirm_placement(&mut self) -> Result<u64, String> {
        let session = self.placement.clone().ok_or("Nenhuma construção em andamento")?;
        let (x, y) = session
            .cursor
            .origin_unsigned()
            .ok_or("Posição fora dos limites da ilha")?;

        let result = match &session.item {
            BuildItem::Habitat(type_id) => self.world.build_habitat(
                type_id,
                x,
                y,
                session.cost,
                &self.habitats_catalog,
            ),
            BuildItem::Farm => self.world.build_farm(x, y, session.cost),
        };

        match result {
            Ok(id) => {
                self.cancel_placement();
                Ok(id)
            }
            Err(e) => {
                // Mantém o modo ativo e atualiza o indicador visual
                let valid = self.placement_is_valid(&session.cursor);
                if let Some(s) = self.placement.as_mut() {
                    s.cursor.set_valid(valid);
                }
                Err(e.to_string())
            }
        }
    }

    /// Abre a loja de construções com habitats elementais e fazenda.
    pub fn open_shop_modal(&mut self) {
        let mut modal = UiModal::new(
            "modal_shop",
            "LOJA DE CONSTRUCOES",
            Rect::new(140.0, 280.0, 800.0, 680.0),
        );
        modal.add_text(
            format!("SALDO: {} OURO", self.world.gold),
            170.0,
            335.0,
            1.8,
            UiColor::GOLD_FILL,
        );

        let mut y = 380.0;
        for hab in self.habitats_catalog.all_sorted() {
            let element = hab
                .supported_elements
                .first()
                .map(|e| e.as_str().to_uppercase())
                .unwrap_or_default();
            let affordable = self.world.gold >= hab.build_cost_gold;
            modal.add_button(
                UiButton::new(
                    format!("shop_habitat:{}", hab.id),
                    Rect::new(170.0, y, 740.0, 54.0),
                    format!("HABITAT {} - {} OURO", element, hab.build_cost_gold),
                )
                .with_colors(
                    if affordable { UiColor::BTN_ACCENT } else { UiColor::BTN_DISABLED },
                    Color::WHITE,
                )
                .with_disabled(!affordable)
                .with_scale(1.8),
            );
            y += 62.0;
        }

        let farm_affordable = self.world.gold >= FARM_BUILD_COST_GOLD;
        modal.add_button(
            UiButton::new(
                "shop_farm",
                Rect::new(170.0, y, 740.0, 54.0),
                format!("FAZENDA DE COMIDA - {} OURO", FARM_BUILD_COST_GOLD),
            )
            .with_colors(
                if farm_affordable { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
                Color::WHITE,
            )
            .with_disabled(!farm_affordable)
            .with_scale(1.8),
        );

        self.ui.open_modal(modal);
    }

    /// Abre o diálogo de limpeza de um obstáculo da ilha.
    pub fn open_obstacle_modal(&mut self, obstacle_id: u64) {
        let Some(obstacle) = self.world.obstacles.get(&obstacle_id).cloned() else { return };
        self.selected_obstacle_id = Some(obstacle_id);

        let cost = obstacle.kind.clear_cost_gold();
        let (w, h) = obstacle.kind.footprint();
        let affordable = self.world.gold >= cost;

        let mut modal = UiModal::new(
            "modal_obstacle",
            "LIMPAR TERRENO",
            Rect::new(140.0, 380.0, 800.0, 420.0),
        );
        modal.add_text(
            format!("OBSTACULO: {} ({}x{} CELULAS)", obstacle.kind.display_name(), w, h),
            170.0,
            450.0,
            1.8,
            Color::WHITE,
        );
        modal.add_text(
            format!("POSICAO NA ILHA: ({}, {})", obstacle.grid_x, obstacle.grid_y),
            170.0,
            490.0,
            1.6,
            Color::rgb(0.95, 0.82, 0.35),
        );
        modal.add_text(
            format!("SALDO: {} OURO", self.world.gold),
            170.0,
            530.0,
            1.6,
            UiColor::GOLD_FILL,
        );
        modal.add_button(
            UiButton::new(
                "btn_clear_obstacle",
                Rect::new(190.0, 600.0, 700.0, 68.0),
                format!("LIMPAR (-{} OURO)", cost),
            )
            .with_colors(
                if affordable { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
                Color::WHITE,
            )
            .with_disabled(!affordable)
            .with_scale(1.9),
        );
        self.ui.open_modal(modal);
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

    /// Inicia uma nova sessão de combate na Arena Elemental contra uma criatura desafiante.
    pub fn start_arena_battle(&mut self) {
        let cid = self.selected_creature_id.or_else(|| self.world.creatures.keys().next().copied());
        let Some(player_cid) = cid else {
            return;
        };
        let Some(creature) = self.world.creatures.get(&player_cid).cloned() else {
            return;
        };
        let Some(player_spec) = self.catalog.get(&creature.species_id).cloned() else {
            return;
        };

        let player_fighter = Combatant::from_creature(&creature, &player_spec);

        // Oponente elemental da Arena (Stone Golem de Terra)
        let enemy_spec = self
            .catalog
            .get("stone_golem")
            .cloned()
            .unwrap_or_else(|| player_spec.clone());
        let mut enemy_instance = game_core::CreatureInstance::new(8888, &enemy_spec.id);
        enemy_instance.level = creature.level;
        let enemy_fighter = Combatant::from_creature(&enemy_instance, &enemy_spec);

        let battle = BattleSession::new(vec![player_fighter], vec![enemy_fighter]);
        self.arena = Some(ArenaSessionState {
            battle,
            player_creature_id: player_cid,
            last_action_log: None,
            reward_gold: 150,
            reward_xp: 80,
            rewards_claimed: false,
        });
        self.open_arena_modal();
    }

    /// Abre ou atualiza o diálogo modal da Arena Elemental de Combate.
    pub fn open_arena_modal(&mut self) {
        let Some(ref arena) = self.arena else {
            self.start_arena_battle();
            return;
        };

        let ally = match arena
            .battle
            .active_player_combatant()
            .cloned()
            .or_else(|| arena.battle.player_team.first().cloned())
        {
            Some(a) => a,
            None => return,
        };
        let enemy = match arena
            .battle
            .active_enemy_combatant()
            .cloned()
            .or_else(|| arena.battle.enemy_team.first().cloned())
        {
            Some(e) => e,
            None => return,
        };

        let mut modal = UiModal::new(
            "modal_arena",
            "ARENA ELEMENTAL DE BATALHA",
            Rect::new(100.0, 200.0, 880.0, 780.0),
        );

        // 1. Painel do Oponente
        let enemy_el = enemy
            .elements
            .iter()
            .map(|e| e.as_str().to_uppercase())
            .collect::<Vec<_>>()
            .join("/");
        modal.add_text(
            format!("OPONENTE: {} [{}]", enemy.name.to_uppercase(), enemy_el),
            130.0,
            260.0,
            1.8,
            Color::rgb(1.0, 0.45, 0.45),
        );
        let enemy_ratio = (enemy.current_hp as f32 / enemy.max_hp.max(1) as f32).clamp(0.0, 1.0);
        modal.add_text(
            format!(
                "HP: {} / {} ({:.0}%) | VELOCIDADE: {}",
                enemy.current_hp,
                enemy.max_hp,
                enemy_ratio * 100.0,
                enemy.speed
            ),
            130.0,
            295.0,
            1.6,
            Color::WHITE,
        );
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(130.0, 325.0, 820.0, 22.0),
            enemy_ratio,
            UiColor::hp_bar_color(enemy_ratio),
        ));

        // 2. Painel do Aliado
        let ally_el = ally
            .elements
            .iter()
            .map(|e| e.as_str().to_uppercase())
            .collect::<Vec<_>>()
            .join("/");
        let initiative_label = if ally.speed >= enemy.speed {
            " (INICIATIVA 1o)"
        } else {
            " (2o NO TURNO)"
        };
        modal.add_text(
            format!(
                "ALIADO: {} [{}]{}",
                ally.name.to_uppercase(),
                ally_el,
                initiative_label
            ),
            130.0,
            370.0,
            1.8,
            Color::rgb(0.45, 0.85, 1.0),
        );
        let ally_ratio = (ally.current_hp as f32 / ally.max_hp.max(1) as f32).clamp(0.0, 1.0);
        modal.add_text(
            format!(
                "HP: {} / {} ({:.0}%) | VELOCIDADE: {}",
                ally.current_hp,
                ally.max_hp,
                ally_ratio * 100.0,
                ally.speed
            ),
            130.0,
            405.0,
            1.6,
            Color::WHITE,
        );
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(130.0, 435.0, 820.0, 22.0),
            ally_ratio,
            UiColor::hp_bar_color(ally_ratio),
        ));

        // 3. Log de Ação do Turno Anterior
        let log_msg = arena
            .last_action_log
            .as_deref()
            .unwrap_or("Escolha uma habilidade elementar para executar o round!");
        modal.add_text(log_msg, 130.0, 480.0, 1.5, UiColor::GOLD_ACCENT);

        // 4. Seção Inferior: Seleção de Habilidades OU Tela de Desfecho
        match arena.battle.outcome {
            BattleOutcome::Ongoing => {
                modal.add_text(
                    format!("TURNO {} - HABILIDADES DE COMBATE:", arena.battle.current_turn),
                    130.0,
                    525.0,
                    1.7,
                    Color::WHITE,
                );

                let mut y_btn = 560.0;
                for (idx, skill) in ally.skills.iter().enumerate() {
                    let cd = ally.skill_cooldowns.get(&skill.id).copied().unwrap_or(0);
                    let is_ready = cd == 0;
                    let tactical_tag = self
                        .elemental_chart
                        .tactical_label(&skill.element, &enemy.elements);

                    let label = if is_ready {
                        format!(
                            "{}: PODER {} [{}] | {}",
                            skill.name.to_uppercase().replace("Á", "A").replace("Ã", "A"),
                            skill.base_power,
                            skill.element.as_str().to_uppercase(),
                            tactical_tag
                        )
                    } else {
                        format!(
                            "{}: RECARGA ({} TURNOS)",
                            skill.name.to_uppercase().replace("Á", "A").replace("Ã", "A"),
                            cd
                        )
                    };

                    let btn = UiButton::new(
                        format!("arena_skill:{}", idx),
                        Rect::new(130.0, y_btn, 820.0, 56.0),
                        label,
                    )
                    .with_colors(
                        if is_ready {
                            UiColor::BTN_PRIMARY
                        } else {
                            UiColor::BTN_DISABLED
                        },
                        Color::WHITE,
                    )
                    .with_disabled(!is_ready)
                    .with_scale(1.7);

                    modal.add_button(btn);
                    y_btn += 66.0;
                }
            }
            BattleOutcome::PlayerVictory => {
                modal.add_text(
                    "VITORIA GLORIOSA NA ARENA!",
                    130.0,
                    530.0,
                    2.2,
                    UiColor::GOLD_ACCENT,
                );
                modal.add_text(
                    format!(
                        "RECOMPENSAS: +{} OURO  |  +{} XP DE BATALHA",
                        arena.reward_gold, arena.reward_xp
                    ),
                    130.0,
                    575.0,
                    1.8,
                    Color::WHITE,
                );

                if !arena.rewards_claimed {
                    modal.add_button(
                        UiButton::new(
                            "arena_claim_rewards",
                            Rect::new(150.0, 630.0, 780.0, 68.0),
                            "REIVINDICAR RECOMPENSAS",
                        )
                        .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                        .with_scale(2.0),
                    );
                } else {
                    modal.add_text(
                        "Recompensas depositadas com sucesso no Santuario e na Criatura!",
                        130.0,
                        625.0,
                        1.6,
                        UiColor::XP_FILL,
                    );
                    modal.add_button(
                        UiButton::new(
                            "arena_close",
                            Rect::new(150.0, 675.0, 780.0, 64.0),
                            "CONCLUIR E RETORNAR AO SANTUARIO",
                        )
                        .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                        .with_scale(1.9),
                    );
                }
            }
            BattleOutcome::EnemyVictory | BattleOutcome::Draw => {
                let msg = if arena.battle.outcome == BattleOutcome::Draw {
                    "EMPATE! COMBATE TERMINOU SEM VENCEDOR"
                } else {
                    "DERROTA! SUA CRIATURA FOI DERROTADA..."
                };
                modal.add_text(
                    msg,
                    130.0,
                    530.0,
                    2.2,
                    UiColor::BTN_DANGER,
                );
                modal.add_text(
                    "Alimente e evolua suas criaturas para superar este desafio elemental.",
                    130.0,
                    575.0,
                    1.6,
                    Color::WHITE,
                );
                modal.add_button(
                    UiButton::new(
                        "arena_close",
                        Rect::new(150.0, 640.0, 780.0, 68.0),
                        "RETORNAR AO SANTUARIO",
                    )
                    .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                    .with_scale(1.9),
                );
            }
        }

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

        // 1. Camada de Fundo (Ground): grade completa da ilha em tiles isométricos
        if let Some(grass) = self.atlas.get("tile_island_grass_01") {
            let tile_w = self.projection.tile_width * self.camera.zoom;
            let tile_h = self.projection.tile_height * self.camera.zoom;
            for x in 0..self.world.island_grid.width {
                for y in 0..self.world.island_grid.height {
                    let screen_pos = self.camera.grid_to_screen(x as f32, y as f32, &self.projection);
                    let tile_rect = Rect::new(screen_pos.x - tile_w * 0.5, screen_pos.y, tile_w, tile_h);
                    self.render_queue.push_ground(grass.clone(), tile_rect, Color::WHITE);
                }
            }
        }

        // 2. Camada de Decalques no Chão (GroundDecal): indicador verde/vermelho do modo de construção
        if let Some(session) = &self.placement {
            session
                .cursor
                .push_overlay(&mut self.render_queue, &self.camera, &self.projection);
        }

        // 3. Camada de Entidades com Y-Sorting (Entities): obstáculos, construções e criaturas
        let zoom = self.camera.zoom;
        let tile_w = self.projection.tile_width * zoom;

        // - Obstáculos naturais (placeholders coloridos até os sprites PNG existirem)
        for obstacle in self.world.obstacles.values() {
            let (w, h) = obstacle.kind.footprint();
            let center = self.camera.grid_to_screen(
                obstacle.grid_x as f32 + w as f32 * 0.5,
                obstacle.grid_y as f32 + h as f32 * 0.5,
                &self.projection,
            );
            let (size, color) = match obstacle.kind {
                ObstacleKind::Bush => (0.45, Color::rgb(0.18, 0.55, 0.22)),
                ObstacleKind::SmallRock => (0.45, Color::rgb(0.52, 0.52, 0.56)),
                ObstacleKind::Tree => (0.55, Color::rgb(0.10, 0.40, 0.16)),
                ObstacleKind::LargeRock => (0.45, Color::rgb(0.38, 0.38, 0.44)),
            };
            let side = tile_w * size * (w as f32);
            let depth = obstacle.grid_x as f32 + obstacle.grid_y as f32 + (w + h) as f32 * 0.5;
            self.render_queue.push(RenderCommand::colored_rect(
                RenderLayer::Entities,
                depth,
                Rect::new(center.x - side * 0.5, center.y - side * 0.9, side, side),
                color,
            ));
        }

        // - Habitats e fazendas construídos (sprite compartilhado até as artes finais)
        if let Some(hab_sprite) = self.atlas.get("habitat_fire_t1") {
            for hab in self.world.habitats.values() {
                let Some(hab_type) = self.habitats_catalog.get(&hab.type_id) else { continue };
                let center = self.camera.grid_to_screen(
                    hab.grid_x as f32 + hab_type.width as f32 * 0.5,
                    hab.grid_y as f32 + hab_type.height as f32 * 0.5,
                    &self.projection,
                );
                let side = (hab_type.width + hab_type.height) as f32 * 0.5 * tile_w * 0.6;
                self.render_queue.push_entity_grid(
                    hab.grid_x as f32 + hab_type.width as f32 * 0.5,
                    hab.grid_y as f32 + hab_type.height as f32 * 0.5,
                    0.0,
                    hab_sprite.clone(),
                    Rect::new(center.x - side * 0.5, center.y - side * 0.85, side, side),
                    Color::WHITE,
                );
            }
        }
        if let Some(farm_sprite) = self.atlas.get("building_farm_empty") {
            for farm in self.world.farms.values() {
                let center = self.camera.grid_to_screen(
                    farm.grid_x as f32 + farm.width as f32 * 0.5,
                    farm.grid_y as f32 + farm.height as f32 * 0.5,
                    &self.projection,
                );
                let side = (farm.width + farm.height) as f32 * 0.5 * tile_w * 0.6;
                self.render_queue.push_entity_grid(
                    farm.grid_x as f32 + farm.width as f32 * 0.5,
                    farm.grid_y as f32 + farm.height as f32 * 0.5,
                    0.0,
                    farm_sprite.clone(),
                    Rect::new(center.x - side * 0.5, center.y - side * 0.85, side, side),
                    Color::WHITE,
                );
            }
        }

        // - Criatura A (grid 2.5, 2.5 -> depth menor)
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

        // - Criatura B na frente do habitat (grid 4.0, 4.0 -> depth maior que o centro do habitat)
        if let Some(pyro) = self.atlas.get("creature_pyro_drake_baby") {
            let p = self.camera.grid_to_screen(4.0, 4.0, &self.projection);
            self.render_queue.push_entity_grid(
                4.0,
                4.0,
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

        // 6. Painel de informação do modo de construção (acima dos botões flutuantes)
        if let Some(session) = &self.placement {
            self.render_queue.push_ui_rect(
                Rect::new(60.0, 1620.0, 960.0, 100.0),
                UiColor::DARK_GLASS,
            );
            self.render_queue.flush_to_batch(&mut self.batch);
            let status = if session.cursor.valid { "AREA LIVRE" } else { "AREA BLOQUEADA" };
            let status_color = if session.cursor.valid {
                Color::rgb(0.4, 0.95, 0.45)
            } else {
                Color::rgb(1.0, 0.4, 0.4)
            };
            self.font.draw_text(
                &mut self.batch,
                &format!(
                    "CONSTRUIR: {} - {} OURO",
                    session.label.to_uppercase().replace("Á", "A").replace("Ç", "C"),
                    session.cost
                ),
                90.0,
                1636.0,
                1.8,
                Color::WHITE,
            );
            self.font.draw_text(
                &mut self.batch,
                &format!("{} - TOQUE NO MAPA PARA MOVER", status),
                90.0,
                1676.0,
                1.6,
                status_color,
            );
        }

        // 7. Renderização da Interface e Modais via UiManager
        self.ui.render(&mut self.render_queue, &mut self.batch, &self.font);
    }

    fn on_resume(&mut self) {
        println!("Bestigard retomado (foreground mobile)");
    }

    fn on_input(&mut self, input: &InputManager) {
        for tap in input.taps() {
            let mut handled_tap = false;
            let action = self.ui.handle_tap(tap);
            match action {
                UiAction::ButtonClicked(ref id) => {
                    handled_tap = true;
                    match id.as_str() {
                        "btn_open_shop" => {
                            self.open_shop_modal();
                        }
                        "shop_farm" => {
                            self.ui.close_modal();
                            self.start_placement(BuildItem::Farm);
                            println!("Construção: modo de posicionamento iniciado (Fazenda).");
                        }
                        id if id.starts_with("shop_habitat:") => {
                            let type_id = id["shop_habitat:".len()..].to_string();
                            self.ui.close_modal();
                            println!("Construção: modo de posicionamento iniciado ('{}').", type_id);
                            self.start_placement(BuildItem::Habitat(type_id));
                        }
                        "btn_placement_confirm" => match self.confirm_placement() {
                            Ok(new_id) => println!("Construção: estrutura #{} construída com sucesso!", new_id),
                            Err(e) => println!("Construção: não foi possível construir: {}", e),
                        },
                        "btn_placement_cancel" => {
                            self.cancel_placement();
                            println!("Construção: modo de posicionamento cancelado.");
                        }
                        "btn_clear_obstacle" => {
                            if let Some(oid) = self.selected_obstacle_id.take() {
                                match self.world.clear_obstacle(oid) {
                                    Ok(cost) => {
                                        println!("Terreno: obstáculo #{} limpo por {} ouro.", oid, cost);
                                        self.ui.close_modal();
                                    }
                                    Err(e) => println!("Terreno: não foi possível limpar: {}", e),
                                }
                            }
                        }
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
                        "btn_open_arena" => {
                            self.start_arena_battle();
                        }
                        id if id.starts_with("arena_skill:") => {
                            let idx_str = &id["arena_skill:".len()..];
                            if let Ok(skill_idx) = idx_str.parse::<usize>() {
                                if let Some(ref mut arena) = self.arena {
                                    let (log1, log2) = arena
                                        .battle
                                        .execute_round(skill_idx, &self.elemental_chart);
                                    let mut summary = String::new();
                                    if let Some(l1) = log1 {
                                        summary.push_str(&format!(
                                            "{}: {} ({} dano, {:.1}x)",
                                            l1.attacker_name,
                                            l1.skill_name,
                                            l1.raw_damage,
                                            l1.elemental_multiplier
                                        ));
                                    }
                                    if let Some(l2) = log2 {
                                        if !summary.is_empty() {
                                            summary.push_str(" | ");
                                        }
                                        summary.push_str(&format!(
                                            "{}: {} ({} dano)",
                                            l2.attacker_name, l2.skill_name, l2.raw_damage
                                        ));
                                    }
                                    arena.last_action_log = Some(summary);
                                }
                                self.open_arena_modal();
                            }
                        }
                        "arena_claim_rewards" => {
                            if let Some(ref mut arena) = self.arena {
                                if !arena.rewards_claimed {
                                    arena.rewards_claimed = true;
                                    let gold = arena.reward_gold;
                                    let xp = arena.reward_xp;
                                    let cid = arena.player_creature_id;
                                    self.world.gold += gold;
                                    if let Some(creature) = self.world.creatures.get_mut(&cid) {
                                        creature.add_xp(xp);
                                    }
                                    println!(
                                        "Arena: Recompensas reivindicadas! +{} ouro, +{} XP na Criatura #{}.",
                                        gold, xp, cid
                                    );
                                }
                            }
                            self.open_arena_modal();
                        }
                        "arena_close" => {
                            self.arena = None;
                            self.ui.close_modal();
                            println!("Arena: Batalha concluida. Retornando ao santuario.");
                        }
                        _ => {}
                    }
                }
                UiAction::ModalClosed => {
                    handled_tap = true;
                    if self.arena.is_some() {
                        self.arena = None;
                        println!("UI: Arena encerrada ao fechar modal.");
                    } else {
                        println!("UI: Modal fechado pelo usuário.");
                    }
                }
                UiAction::None => {}
            }

            if !handled_tap && !self.ui.is_modal_open() {
                let cell = self.camera.screen_to_grid_cell(tap.x, tap.y, &self.projection);
                println!(
                    "Entrada Unificada: Tap capturado em ({:.0}, {:.0}) -> Célula da ilha selecionada: ({}, {})",
                    tap.x, tap.y, cell.0, cell.1
                );

                // Modo de construção: o toque reposiciona o cursor sobre o grid
                if self.placement.is_some() {
                    self.move_placement_to(cell);
                    continue;
                }

                // Toque sobre obstáculo: abre diálogo de limpeza
                if cell.0 >= 0 && cell.1 >= 0 {
                    if let Some(oid) = self
                        .world
                        .obstacle_at(cell.0 as u32, cell.1 as u32)
                        .map(|o| o.instance_id)
                    {
                        println!("Terreno: obstáculo #{} selecionado no mapa -> Abrindo menu de limpeza", oid);
                        self.open_obstacle_modal(oid);
                        continue;
                    }
                }

                // Se tocou sobre um habitat, abre modal do habitat
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
        // (inclusive durante o modo de construção, para localizar a área desejada)
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
    println!("1. Testando toque no botão de HUD [COLETAR OURO] (pos: 500, 40)...");
    let mut hud_input = InputManager::new();
    hud_input.on_mouse_button_down(MouseButton::Left, 500.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 500.0, 40.0);
    state.on_input(&hud_input);
    hud_input.clear_events();

    // B) Toque no botão global do HUD: [CRIATURA] para abrir modal de inspeção
    println!("\n2. Abrindo diálogo modal de Gestão da Criatura Ignis [CRIATURA] (pos: 620, 40)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 620.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 620.0, 40.0);
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

    println!("Abrindo modal do [CRUZAMENTO] (pos: 740, 40)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 740.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 740.0, 40.0);
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

    // 9. Modo de Construção Interativo e Limpeza de Obstáculos
    println!("\n--- MODO DE CONSTRUÇÃO & LIMPEZA DE OBSTÁCULOS ---");
    let tap_at = |state: &mut BestigardGame, x: f32, y: f32| {
        let mut input = InputManager::new();
        input.on_mouse_button_down(MouseButton::Left, x, y);
        input.on_mouse_button_up(MouseButton::Left, x, y);
        state.on_input(&input);
    };
    // Converte o centro de uma célula do grid em coordenadas de tela com a câmera atual
    let cell_to_screen = |state: &BestigardGame, cx: u32, cy: u32| {
        let p = state
            .camera
            .grid_to_screen(cx as f32 + 0.5, cy as f32 + 0.5, &state.projection);
        (p.x, p.y)
    };

    let habitats_before = state.world.habitats.len();
    let gold_before = state.world.gold;

    println!("1. Tocando em [LOJA] (pos: 860, 40) e escolhendo [HABITAT WATER]...");
    tap_at(state, 860.0, 40.0);
    assert!(state.ui.is_modal_open());
    tap_at(state, 400.0, 780.0);
    assert!(state.placement.is_some());
    assert!(!state.ui.is_modal_open());
    println!("Modo de posicionamento ativo (HUD substituído por [CONFIRMAR]/[CANCELAR]).");

    println!("2. Movendo cursor sobre o habitat existente (célula 3,3) -> área deve ficar BLOQUEADA...");
    let (sx, sy) = cell_to_screen(state, 3, 3);
    tap_at(state, sx, sy);
    assert!(!state.placement.as_ref().unwrap().cursor.valid);
    tap_at(state, 340.0, 1788.0); // [CONFIRMAR] com área inválida não constrói
    assert_eq!(state.world.habitats.len(), habitats_before);
    assert!(state.placement.is_some());

    println!("3. Movendo cursor para área livre (célula 8,6) -> área deve ficar LIVRE...");
    let (sx, sy) = cell_to_screen(state, 8, 6);
    tap_at(state, sx, sy);
    assert!(state.placement.as_ref().unwrap().cursor.valid);

    println!("4. Confirmando construção em [CONFIRMAR] (pos: 340, 1788)...");
    tap_at(state, 340.0, 1788.0);
    assert!(state.placement.is_none());
    assert_eq!(state.world.habitats.len(), habitats_before + 1);
    assert_eq!(state.world.gold, gold_before - 300);
    println!(
        "Habitat construído! Ouro: {} -> {}. Total de habitats: {}.",
        gold_before,
        state.world.gold,
        state.world.habitats.len()
    );

    println!("5. Cancelamento: abrindo [LOJA] -> [FAZENDA] e cancelando em [CANCELAR] (pos: 740, 1788)...");
    tap_at(state, 860.0, 40.0);
    tap_at(state, 400.0, 840.0);
    assert!(state.placement.is_some());
    let gold_after_build = state.world.gold;
    tap_at(state, 740.0, 1788.0);
    assert!(state.placement.is_none());
    assert_eq!(state.world.gold, gold_after_build);

    println!("6. Limpando obstáculo: tocando na Pedra Pequena (célula 10,3)...");
    let obstacles_before = state.world.obstacles.len();
    let gold_before_clear = state.world.gold;
    let (sx, sy) = cell_to_screen(state, 10, 3);
    tap_at(state, sx, sy);
    assert!(state.ui.is_modal_open());
    tap_at(state, 400.0, 634.0); // [LIMPAR]
    assert_eq!(state.world.obstacles.len(), obstacles_before - 1);
    assert_eq!(state.world.gold, gold_before_clear - 25);
    assert!(state.world.can_place_building(10, 3, 1, 1).is_ok());
    assert!(!state.ui.is_modal_open());
    println!(
        "Obstáculo removido por 25 ouro. Obstáculos restantes: {}. Célula (10,3) liberada para construção.",
        state.world.obstacles.len()
    );

    // 10. Demonstração e Verificação da Interface de Combate Elemental e Arena de Batalha (Passo 5.4)
    println!("\n--- ARENA ELEMENTAL DE BATALHA & UI INTERATIVA (PASSO 5.4) ---");
    let gold_before_arena = state.world.gold;

    println!("1. Abrindo Arena Elemental de Batalha via botão [ARENA] no HUD (pos: 980, 40)...");
    tap_at(state, 980.0, 40.0);
    assert!(state.ui.is_modal_open());
    assert!(state.arena.is_some());
    let arena_state = state.arena.as_ref().unwrap();
    assert_eq!(arena_state.battle.current_turn, 1);
    assert_eq!(arena_state.battle.outcome, BattleOutcome::Ongoing);
    println!(
        "Arena iniciada! Combatentes: Aliado '{}' (HP {}) vs Inimigo '{}' (HP {})",
        arena_state.battle.player_team.first().unwrap().name,
        arena_state.battle.player_team.first().unwrap().current_hp,
        arena_state.battle.enemy_team.first().unwrap().name,
        arena_state.battle.enemy_team.first().unwrap().current_hp,
    );

    println!("2. Executando habilidade de ataque [GOLPE DE FOGO] (pos: 400, 588)...");
    tap_at(state, 400.0, 588.0);
    assert!(state.arena.is_some());
    println!(
        "Log de combate após round 1: {:?}",
        state.arena.as_ref().unwrap().last_action_log
    );

    println!("3. Executando habilidade super-efetiva até desfecho da batalha...");
    while state.arena.as_ref().unwrap().battle.outcome == BattleOutcome::Ongoing {
        tap_at(state, 400.0, 588.0);
    }

    assert_eq!(
        state.arena.as_ref().unwrap().battle.outcome,
        BattleOutcome::PlayerVictory
    );
    println!(
        "Vitória alcançada na Arena! Desfecho: {:?}",
        state.arena.as_ref().unwrap().battle.outcome
    );

    println!("4. Reivindicando recompensas de ouro e XP no diálogo de vitória (pos: 400, 664)...");
    let fighter_id = state.arena.as_ref().unwrap().player_creature_id;
    let fighter_level_before = state.world.creatures.get(&fighter_id).unwrap().level;
    tap_at(state, 400.0, 664.0);
    assert!(state.arena.as_ref().unwrap().rewards_claimed);
    assert_eq!(state.world.gold, gold_before_arena + 150);
    let fighter_after = state.world.creatures.get(&fighter_id).unwrap();
    assert!(
        fighter_after.level > fighter_level_before || fighter_after.xp > 0,
        "XP ou Nível da criatura combatente deve ter progredido com a recompensa"
    );
    println!(
        "Recompensas injetadas! Ouro do Santuário: {} -> {}. Criatura #{}: Nível {} (XP {}).",
        gold_before_arena,
        state.world.gold,
        fighter_id,
        fighter_after.level,
        fighter_after.xp,
    );

    println!("5. Concluindo duelo e fechando modal de Arena (pos: 400, 707)...");
    tap_at(state, 400.0, 707.0);
    assert!(state.arena.is_none());
    assert!(!state.ui.is_modal_open());
    println!("Arena fechada e santuário restaurado com sucesso!");

    println!("\n>> Interface de Usuário (HUD e Menus Modais) validada com 100% de sucesso! <<");
    println!(">> Modo de Construção e Limpeza de Terreno validados com 100% de sucesso! <<");
    println!(">> Arena Elemental de Batalha e UI Interativa validadas com 100% de sucesso! <<");
}
