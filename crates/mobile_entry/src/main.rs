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

/// Aba ativa no menu de compras da Loja do Santuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShopTab {
    #[default]
    Buildings,
    Eggs,
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
    pub selected_farm_id: Option<u64>,
    pub selected_parent1_id: Option<u64>,
    pub selected_parent2_id: Option<u64>,
    pub shop_tab: ShopTab,
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
            selected_farm_id: None,
            selected_parent1_id: None,
            selected_parent2_id: None,
            shop_tab: ShopTab::Buildings,
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
            UiButton::new("btn_collect_all", Rect::new(360.0, 24.0, 92.0, 56.0), "COLETAR")
                .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                .with_scale(1.6),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_creature", Rect::new(458.0, 24.0, 92.0, 56.0), "CRIATURA")
                .with_colors(UiColor::BTN_ACCENT, Color::WHITE)
                .with_scale(1.6),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_breeding", Rect::new(556.0, 24.0, 92.0, 56.0), "CRUZAR")
                .with_colors(UiColor::GOLD_ACCENT, Color::rgb(0.08, 0.12, 0.22))
                .with_scale(1.6),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_hatchery", Rect::new(654.0, 24.0, 92.0, 56.0), "NINHO")
                .with_colors(Color::rgb(0.85, 0.55, 0.20), Color::WHITE)
                .with_scale(1.6),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_bestiary", Rect::new(752.0, 24.0, 92.0, 56.0), "BESTIARIO")
                .with_colors(Color::rgb(0.40, 0.25, 0.65), Color::WHITE)
                .with_scale(1.5),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_shop", Rect::new(850.0, 24.0, 92.0, 56.0), "LOJA")
                .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                .with_scale(1.6),
        );
        self.ui.add_global_button(
            UiButton::new("btn_open_arena", Rect::new(948.0, 24.0, 95.0, 56.0), "ARENA")
                .with_colors(UiColor::BTN_DANGER, Color::WHITE)
                .with_scale(1.6),
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

    /// Abre a loja de compras com abas para construções e ovos elementais.
    pub fn open_shop_modal(&mut self) {
        let mut modal = UiModal::new(
            "modal_shop",
            "LOJA DO SANTUARIO",
            Rect::new(140.0, 240.0, 800.0, 720.0),
        );
        modal.add_text(
            format!("SALDO: {} OURO", self.world.gold),
            170.0,
            295.0,
            1.8,
            UiColor::GOLD_FILL,
        );

        // Abas da loja
        let is_buildings = self.shop_tab == ShopTab::Buildings;
        let is_eggs = self.shop_tab == ShopTab::Eggs;

        modal.add_button(
            UiButton::new(
                "btn_shop_tab:buildings",
                Rect::new(170.0, 330.0, 360.0, 48.0),
                "ESTRUTURAS",
            )
            .with_colors(
                if is_buildings { UiColor::BTN_PRIMARY } else { UiColor::BTN_SECONDARY },
                Color::WHITE,
            )
            .with_scale(1.8),
        );

        modal.add_button(
            UiButton::new(
                "btn_shop_tab:eggs",
                Rect::new(550.0, 330.0, 360.0, 48.0),
                "OVOS ELEMENTAIS",
            )
            .with_colors(
                if is_eggs { UiColor::BTN_PRIMARY } else { UiColor::BTN_SECONDARY },
                Color::WHITE,
            )
            .with_scale(1.8),
        );

        match self.shop_tab {
            ShopTab::Buildings => {
                let mut y = 395.0;
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
                            Rect::new(170.0, y, 740.0, 52.0),
                            format!("HABITAT {} - {} OURO", element, hab.build_cost_gold),
                        )
                        .with_colors(
                            if affordable { UiColor::BTN_ACCENT } else { UiColor::BTN_DISABLED },
                            Color::WHITE,
                        )
                        .with_disabled(!affordable)
                        .with_scale(1.7),
                    );
                    y += 58.0;
                }

                let farm_affordable = self.world.gold >= FARM_BUILD_COST_GOLD;
                modal.add_button(
                    UiButton::new(
                        "shop_farm",
                        Rect::new(170.0, y, 740.0, 52.0),
                        format!("FAZENDA DE COMIDA - {} OURO", FARM_BUILD_COST_GOLD),
                    )
                    .with_colors(
                        if farm_affordable { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
                        Color::WHITE,
                    )
                    .with_disabled(!farm_affordable)
                    .with_scale(1.7),
                );
            }
            ShopTab::Eggs => {
                let hatchery_has_space = self.world.hatchery.can_add_egg();
                let hatchery_cap = self.world.hatchery.max_slots;
                let eggs_count = self.world.hatchery.eggs.len();

                modal.add_text(
                    format!(
                        "NINHO DE INCUBACAO: {} / {} VAGAS OCUPADAS",
                        eggs_count, hatchery_cap
                    ),
                    170.0,
                    395.0,
                    1.6,
                    if hatchery_has_space { Color::rgb(0.4, 0.9, 0.4) } else { UiColor::BTN_DANGER },
                );

                let mut y = 430.0;
                for (species, cost) in self.catalog.shop_eggs_sorted() {
                    let can_afford = self.world.gold >= cost;
                    let can_buy = can_afford && hatchery_has_space;
                    let el = species.elements.first().map(|e| e.as_str().to_uppercase()).unwrap_or_default();

                    modal.add_button(
                        UiButton::new(
                            format!("shop_buy_egg:{}", species.id),
                            Rect::new(170.0, y, 740.0, 54.0),
                            format!("OVO {} ({}) - {} OURO", species.name.to_uppercase(), el, cost),
                        )
                        .with_colors(
                            if can_buy { UiColor::BTN_ACCENT } else { UiColor::BTN_DISABLED },
                            Color::WHITE,
                        )
                        .with_disabled(!can_buy)
                        .with_scale(1.6),
                    );
                    y += 60.0;
                }
            }
        }

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

    /// Abre janela modal de inspeção e gestão de criatura (navegação, alimentação, nível, evolução e realocação).
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
            Rect::new(140.0, 240.0, 800.0, 720.0),
        );

        // Lista ordenada de todas as criaturas do santuário para permitir navegação cíclica
        let mut all_ids: Vec<u64> = self.world.creatures.keys().copied().collect();
        all_ids.sort();
        let current_pos = all_ids.iter().position(|&id| id == creature_id).unwrap_or(0);
        let prev_id = if current_pos > 0 {
            all_ids[current_pos - 1]
        } else {
            *all_ids.last().unwrap_or(&creature_id)
        };
        let next_id = if current_pos + 1 < all_ids.len() {
            all_ids[current_pos + 1]
        } else {
            all_ids[0]
        };

        // Barra superior de navegação
        modal.add_button(
            UiButton::new(
                format!("creature_nav:{}", prev_id),
                Rect::new(170.0, 290.0, 160.0, 44.0),
                "< ANTERIOR",
            )
            .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
            .with_scale(1.5),
        );

        let nick = creature.nickname.as_deref().unwrap_or(&species.name);
        modal.add_text(
            format!(
                "#{}/{} - {} ('{}')",
                current_pos + 1,
                all_ids.len(),
                species.name.to_uppercase(),
                nick.to_uppercase()
            ),
            350.0,
            303.0,
            1.6,
            UiColor::GOLD_FILL,
        );

        modal.add_button(
            UiButton::new(
                format!("creature_nav:{}", next_id),
                Rect::new(750.0, 290.0, 160.0, 44.0),
                "PROXIMA >",
            )
            .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
            .with_scale(1.5),
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
            350.0,
            1.6,
            Color::rgb(0.95, 0.82, 0.35),
        );

        modal.add_text(
            format!("ESTAGIO: {:?} | NIVEL ATUAL: {}", creature.stage, creature.level),
            170.0,
            385.0,
            1.7,
            Color::rgb(0.35, 0.90, 0.90),
        );

        let hab_label = match creature.habitat_id {
            Some(hid) => format!("#{}", hid),
            None => "NENHUM".to_string(),
        };
        modal.add_text(
            format!(
                "XP: {} / {} | HABITAT ATUAL: {}",
                creature.xp,
                creature.xp_for_next_level(),
                hab_label
            ),
            170.0,
            420.0,
            1.5,
            Color::WHITE,
        );

        // Barra de progresso de experiência (XP)
        let ratio = (creature.xp as f32 / creature.xp_for_next_level().max(1) as f32).clamp(0.0, 1.0);
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(170.0, 445.0, 740.0, 20.0),
            ratio,
            UiColor::XP_FILL,
        ));

        let gold_hr = creature.calculate_gold_per_hour(&species);
        modal.add_text(
            format!("PRODUCAO: {} OURO/HORA | COMIDA DISPONIVEL: {}", gold_hr, self.world.food),
            170.0,
            480.0,
            1.5,
            Color::rgb(1.0, 0.85, 0.2),
        );

        // Seletor de realocação para outro habitat compatível com vaga disponível
        let other_compatible_habitats: Vec<u64> = self
            .world
            .habitats
            .iter()
            .filter_map(|(&hid, hab)| {
                if Some(hid) == creature.habitat_id {
                    return None;
                }
                let hab_type = self.habitats_catalog.get(&hab.type_id)?;
                let tier_info = hab_type.get_tier_info(hab.tier)?;
                if hab.creature_ids.len() < tier_info.max_creatures
                    && hab_type.accepts_elements(&species.elements)
                {
                    Some(hid)
                } else {
                    None
                }
            })
            .collect();

        if let Some(&target_hid) = other_compatible_habitats.first() {
            let target_hab = &self.world.habitats[&target_hid];
            let hab_type = self.habitats_catalog.get(&target_hab.type_id).unwrap();
            modal.add_button(
                UiButton::new(
                    format!("relocate_creature:{}:{}", creature_id, target_hid),
                    Rect::new(170.0, 545.0, 740.0, 52.0),
                    format!("TRANSFERIR P/ HABITAT #{} ({})", target_hid, hab_type.name.to_uppercase()),
                )
                .with_colors(UiColor::BTN_ACCENT, Color::WHITE)
                .with_scale(1.6),
            );
        } else {
            modal.add_text(
                "NENHUM OUTRO HABITAT COMPATIVEL COM VAGA DISPONIVEL",
                170.0,
                560.0,
                1.4,
                Color::rgb(0.7, 0.7, 0.7),
            );
        }

        // Custo dinâmico de alimentação: 20 * level
        let feed_cost = 20 * creature.level as u64;
        let can_feed = self.world.food >= feed_cost;

        modal.add_button(
            UiButton::new(
                "btn_feed_action",
                Rect::new(170.0, 620.0, 740.0, 64.0),
                format!("ALIMENTAR CRIATURA (-{} COMIDA)", feed_cost),
            )
            .with_colors(
                if can_feed { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
                Color::WHITE,
            )
            .with_disabled(!can_feed)
            .with_scale(1.8),
        );

        // Dica de evolução
        let evo_msg = if creature.level < 4 {
            "Evolui para Jovem no Nivel 4 (Pronto para Cruzamento)"
        } else if creature.level < 10 {
            "Evolui para Adulto Titanico no Nivel 10 (Pico de Poder)"
        } else {
            "Estagio Maximo de Adulto Atingido!"
        };
        modal.add_text(evo_msg, 170.0, 695.0, 1.4, UiColor::GOLD_ACCENT);

        self.ui.open_modal(modal);
    }

    /// Abre janela modal do Altar de Cruzamento com escolha manual dos progenitores.
    pub fn open_breeding_modal(&mut self) {
        let mut modal = UiModal::new(
            "modal_breeding",
            "ALTAR DE CRUZAMENTO GENETICO",
            Rect::new(140.0, 240.0, 800.0, 720.0),
        );

        // Identifica criaturas adultas/jovens elegíveis para cruzar (nível >= 4)
        let mut eligible: Vec<u64> = self
            .world
            .creatures
            .iter()
            .filter(|(_, c)| c.can_breed())
            .map(|(&id, _)| id)
            .collect();
        eligible.sort();

        modal.add_text(
            "SELECAO DE PAIS PARA COMBINACAO DE ELEMENTOS",
            170.0,
            295.0,
            1.7,
            Color::WHITE,
        );

        if eligible.len() >= 2 {
            // Valida ou inicializa os IDs selecionados
            if self.selected_parent1_id.is_none() || !eligible.contains(&self.selected_parent1_id.unwrap()) {
                self.selected_parent1_id = Some(eligible[0]);
            }
            if self.selected_parent2_id.is_none()
                || !eligible.contains(&self.selected_parent2_id.unwrap())
                || self.selected_parent1_id == self.selected_parent2_id
            {
                self.selected_parent2_id = eligible.iter().find(|&&id| Some(id) != self.selected_parent1_id).copied();
            }

            let p1_id = self.selected_parent1_id.unwrap();
            let p2_id = self.selected_parent2_id.unwrap();

            let p1 = &self.world.creatures[&p1_id];
            let p2 = &self.world.creatures[&p2_id];
            let s1 = self.catalog.get(&p1.species_id).unwrap();
            let s2 = self.catalog.get(&p2.species_id).unwrap();

            // Pai 1
            let el1_str = s1.elements.iter().map(|e| e.as_str().to_uppercase()).collect::<Vec<_>>().join("/");
            modal.add_text(
                format!("PAI 1: {} (NV {}) [{}]", s1.name.to_uppercase(), p1.level, el1_str),
                170.0,
                350.0,
                1.6,
                Color::rgb(1.0, 0.65, 0.35),
            );
            modal.add_button(
                UiButton::new("btn_cycle_parent1", Rect::new(690.0, 335.0, 220.0, 44.0), "TROCAR PAI 1")
                    .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                    .with_scale(1.5),
            );

            // Pai 2
            let el2_str = s2.elements.iter().map(|e| e.as_str().to_uppercase()).collect::<Vec<_>>().join("/");
            modal.add_text(
                format!("PAI 2: {} (NV {}) [{}]", s2.name.to_uppercase(), p2.level, el2_str),
                170.0,
                410.0,
                1.6,
                Color::rgb(0.55, 0.85, 1.0),
            );
            modal.add_button(
                UiButton::new("btn_cycle_parent2", Rect::new(690.0, 395.0, 220.0, 44.0), "TROCAR PAI 2")
                    .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                    .with_scale(1.5),
            );

            modal.add_text(
                "PREVISAO GENETICA: HIBRIDO ELEMENTAL COMPATIVEL",
                170.0,
                470.0,
                1.6,
                UiColor::GOLD_ACCENT,
            );
            modal.add_text(
                format!(
                    "Criaturas prontas no santuário: {} | Cruzando: #{} e #{}",
                    eligible.len(),
                    p1_id,
                    p2_id
                ),
                170.0,
                510.0,
                1.5,
                Color::WHITE,
            );

            let hatchery_has_space = self.world.hatchery.can_add_egg();
            modal.add_button(
                UiButton::new(
                    "btn_breed_action",
                    Rect::new(170.0, 610.0, 740.0, 68.0),
                    if hatchery_has_space {
                        "INICIAR CRUZAMENTO (GERAR OVO NO NINHO)"
                    } else {
                        "NINHO CHEIO! (LIBERE UMA VAGA)"
                    },
                )
                .with_colors(
                    if hatchery_has_space { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
                    Color::WHITE,
                )
                .with_disabled(!hatchery_has_space)
                .with_scale(1.7),
            );
        } else {
            modal.add_text(
                "CRIATURAS ADULTAS INSUFICIENTES (MINIMO: 2 NIVEL 4+)",
                170.0,
                380.0,
                1.6,
                UiColor::BTN_DANGER,
            );
            modal.add_text(
                "Alimente suas criaturas até o Nível 4 (Juvenil) para liberá-las!",
                170.0,
                430.0,
                1.5,
                Color::WHITE,
            );
            modal.add_text(
                format!("Criaturas prontas: {} / 2 requeridas.", eligible.len()),
                170.0,
                480.0,
                1.6,
                UiColor::GOLD_ACCENT,
            );
        }

        self.ui.open_modal(modal);
    }

    /// Abre janela modal de inspeção, coleta de ouro e melhoria (upgrade) do habitat.
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
            Rect::new(140.0, 240.0, 800.0, 720.0),
        );

        modal.add_text(
            format!("TIPO: {} (TIER {})", hab_type.name.to_uppercase(), hab.tier),
            170.0,
            300.0,
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
            340.0,
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
            380.0,
            1.6,
            Color::WHITE,
        );
        modal.add_text(
            format!(
                "OURO ACUMULADO: {} / {} LIMITE",
                hab.accumulated_gold, gold_cap
            ),
            170.0,
            420.0,
            1.7,
            Color::rgb(1.0, 0.85, 0.2),
        );

        // Barra de acúmulo de ouro
        let ratio = (hab.accumulated_gold as f32 / gold_cap as f32).clamp(0.0, 1.0);
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(170.0, 450.0, 740.0, 22.0),
            ratio,
            UiColor::GOLD_FILL,
        ));

        // Seção de Melhoria (Upgrade de Tier)
        let upgrade_cost_opt = self.world.habitat_upgrade_cost(habitat_id, &self.habitats_catalog);
        if let Some(cost) = upgrade_cost_opt {
            let next_tier = hab.tier + 1;
            let next_tier_info = hab_type.get_tier_info(next_tier);
            let next_cap = next_tier_info.map(|t| t.gold_capacity).unwrap_or(gold_cap);
            let next_creatures = next_tier_info.map(|t| t.max_creatures).unwrap_or(max_creatures);

            modal.add_text(
                format!(
                    "PROXIMO TIER {}: Capacidade {} ouro, {} criaturas",
                    next_tier, next_cap, next_creatures
                ),
                170.0,
                460.0,
                1.5,
                Color::rgb(0.35, 0.90, 0.90),
            );

            let can_upgrade = self.world.gold >= cost;
            modal.add_button(
                UiButton::new(
                    format!("btn_upgrade_hab_action:{}", habitat_id),
                    Rect::new(170.0, 490.0, 740.0, 54.0),
                    format!("MELHORAR HABITAT (TIER {}) - {} OURO", next_tier, cost),
                )
                .with_colors(
                    if can_upgrade { UiColor::BTN_ACCENT } else { UiColor::BTN_DISABLED },
                    Color::WHITE,
                )
                .with_disabled(!can_upgrade)
                .with_scale(1.7),
            );
        } else {
            modal.add_text(
                "HABITAT NO NIVEL MAXIMO (TIER MAXIMO ALCANCADO)",
                170.0,
                470.0,
                1.6,
                Color::rgb(0.5, 0.9, 0.5),
            );
        }

        let has_gold = hab.accumulated_gold > 0;
        let collect_btn = UiButton::new(
            "btn_collect_hab_action",
            Rect::new(170.0, 560.0, 740.0, 64.0),
            format!("COLETAR +{} DE OURO", hab.accumulated_gold),
        )
        .with_colors(
            if has_gold { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
            Color::WHITE,
        )
        .with_disabled(!has_gold)
        .with_scale(1.8);
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

    /// Abre janela modal de gestão da fazenda agrícola (plantio, maturação e colheita).
    pub fn open_farm_modal(&mut self, farm_id: u64) {
        self.selected_farm_id = Some(farm_id);
        let Some(farm) = self.world.farms.get(&farm_id).cloned() else { return };

        let mut modal = UiModal::new(
            "modal_farm",
            "GESTAO DE FAZENDA AGRICOLA",
            Rect::new(140.0, 260.0, 800.0, 720.0),
        );

        modal.add_text(
            format!("FAZENDA #{} - LOCALIZACAO: ({}, {})", farm_id, farm.grid_x, farm.grid_y),
            170.0,
            330.0,
            1.8,
            Color::WHITE,
        );
        modal.add_text(
            format!("SALDO: {} OURO  |  ESTOQUE: {} COMIDA", self.world.gold, self.world.food),
            170.0,
            370.0,
            1.6,
            UiColor::GOLD_FILL,
        );

        match &farm.state {
            game_core::FarmState::Idle => {
                modal.add_text(
                    "STATUS: CANTEIRO LIVRE PARA PLANTIO",
                    170.0,
                    415.0,
                    1.7,
                    Color::rgb(0.4, 0.95, 0.45),
                );
                modal.add_text(
                    "SELECIONE A SEMENTE DESEJADA:",
                    170.0,
                    455.0,
                    1.6,
                    Color::WHITE,
                );

                let mut y_btn = 495.0;
                for crop in self.crops_catalog.all_sorted() {
                    let affordable = self.world.gold >= crop.gold_cost;
                    let label = format!(
                        "{}: {} OURO ({}S -> +{} COMIDA)",
                        crop.name.to_uppercase().replace("Á", "A").replace("Í", "I"),
                        crop.gold_cost,
                        crop.growth_time_secs,
                        crop.food_yield
                    );

                    modal.add_button(
                        UiButton::new(
                            format!("plant_crop:{}:{}", farm_id, crop.id),
                            Rect::new(170.0, y_btn, 740.0, 50.0),
                            label,
                        )
                        .with_colors(
                            if affordable { UiColor::BTN_PRIMARY } else { UiColor::BTN_DISABLED },
                            Color::WHITE,
                        )
                        .with_disabled(!affordable)
                        .with_scale(1.7),
                    );
                    y_btn += 56.0;
                }
            }
            game_core::FarmState::Growing {
                crop_id,
                elapsed_secs,
                duration_secs,
                food_yield,
            } => {
                let crop_name = self
                    .crops_catalog
                    .get(crop_id)
                    .map(|c| c.name.to_uppercase().replace("Á", "A").replace("Í", "I"))
                    .unwrap_or_else(|| crop_id.to_uppercase());
                let remaining = (*duration_secs as f64 - elapsed_secs).max(0.0).ceil() as u64;
                let ratio = (*elapsed_secs as f32 / *duration_secs as f32).clamp(0.0, 1.0);

                modal.add_text(
                    format!("STATUS: CULTIVANDO {}", crop_name),
                    170.0,
                    420.0,
                    1.8,
                    Color::rgb(0.4, 0.85, 1.0),
                );
                modal.add_text(
                    format!(
                        "PROGRESSO: {:.0}s / {}s (RESTAM {}s)",
                        elapsed_secs, duration_secs, remaining
                    ),
                    170.0,
                    465.0,
                    1.6,
                    Color::WHITE,
                );

                modal.add_progress_bar(UiProgressBar::new(
                    Rect::new(170.0, 505.0, 740.0, 24.0),
                    ratio,
                    Color::rgb(0.35, 0.85, 0.35),
                ));

                modal.add_text(
                    format!("PRODUCAO ESPERADA NA COLHEITA: +{} COMIDA", food_yield),
                    170.0,
                    555.0,
                    1.6,
                    UiColor::GOLD_ACCENT,
                );
                modal.add_text(
                    "O tempo de cultivo avança continuamente mesmo fora do jogo!",
                    170.0,
                    600.0,
                    1.5,
                    Color::WHITE,
                );
            }
            game_core::FarmState::ReadyToHarvest {
                crop_id,
                food_yield,
            } => {
                let crop_name = self
                    .crops_catalog
                    .get(crop_id)
                    .map(|c| c.name.to_uppercase().replace("Á", "A").replace("Í", "I"))
                    .unwrap_or_else(|| crop_id.to_uppercase());

                modal.add_text(
                    format!("STATUS: {} MADURA!", crop_name),
                    170.0,
                    420.0,
                    2.0,
                    UiColor::GOLD_ACCENT,
                );
                modal.add_text(
                    format!("PRODUCAO PRONTA: +{} COMIDA NUTRITIVA", food_yield),
                    170.0,
                    470.0,
                    1.8,
                    Color::rgb(0.4, 0.95, 0.45),
                );

                modal.add_button(
                    UiButton::new(
                        format!("harvest_farm:{}", farm_id),
                        Rect::new(190.0, 560.0, 700.0, 72.0),
                        format!("COLHER +{} DE COMIDA", food_yield),
                    )
                    .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                    .with_scale(2.0),
                );
            }
        }

        self.ui.open_modal(modal);
    }

    /// Abre janela modal do Ninho de Incubação (choca, aceleradores e eclosão).
    pub fn open_hatchery_modal(&mut self) {
        let mut modal = UiModal::new(
            "modal_hatchery",
            "NINHO DE INCUBACAO DE OVOS",
            Rect::new(120.0, 200.0, 840.0, 800.0),
        );

        let egg_count = self.world.hatchery.eggs.len();
        let max_slots = self.world.hatchery.max_slots;
        let speed_mult = self.world.hatchery.total_speed_multiplier();

        modal.add_text(
            format!("OCUPACAO: {} / {} SLOTS DE OVOS", egg_count, max_slots),
            150.0,
            270.0,
            1.8,
            Color::WHITE,
        );
        modal.add_text(
            format!("VELOCIDADE ATUAL: {:.1}x (MODIFICADORES ATIVOS: {})", speed_mult, self.world.hatchery.modifiers.len()),
            150.0,
            310.0,
            1.6,
            UiColor::GOLD_ACCENT,
        );

        if self.world.hatchery.eggs.is_empty() {
            modal.add_text(
                "NENHUM OVO EM INCUBACAO NO MOMENTO.",
                150.0,
                380.0,
                1.8,
                Color::rgb(0.85, 0.85, 0.85),
            );
            modal.add_text(
                "Dica: Compre ovos mono-elementais na Loja ou cruze",
                150.0,
                425.0,
                1.5,
                Color::WHITE,
            );
            modal.add_text(
                "criaturas de nivel 4+ no Altar para gerar hibridos unicos!",
                150.0,
                455.0,
                1.5,
                Color::WHITE,
            );
        } else {
            let mut y_egg = 360.0;
            let eggs_snapshot = self.world.hatchery.eggs.clone();
            for egg in &eggs_snapshot {
                let spec = self.catalog.get(&egg.species_id);
                let name = spec
                    .map(|s| s.name.to_uppercase().replace("Á", "A").replace("Í", "I").replace("Â", "A"))
                    .unwrap_or_else(|| egg.species_id.to_uppercase());
                let rarity = spec.map(|s| format!("{:?}", s.rarity)).unwrap_or_else(|| "Comum".into());

                modal.add_text(
                    format!("OVO #{}: {} [{}]", egg.egg_id, name, rarity.to_uppercase()),
                    150.0,
                    y_egg,
                    1.7,
                    Color::rgb(1.0, 0.75, 0.35),
                );

                let ratio = (egg.elapsed_secs as f32 / egg.total_duration_secs.max(1) as f32).clamp(0.0, 1.0);
                modal.add_progress_bar(UiProgressBar::new(
                    Rect::new(150.0, y_egg + 26.0, 780.0, 18.0),
                    ratio,
                    if egg.is_ready() { Color::rgb(0.35, 0.95, 0.45) } else { UiColor::GOLD_FILL },
                ));

                if egg.is_ready() {
                    modal.add_text(
                        "STATUS: PRONTO PARA ECLODIR!",
                        150.0,
                        y_egg + 56.0,
                        1.5,
                        Color::rgb(0.4, 0.95, 0.45),
                    );
                    modal.add_button(
                        UiButton::new(
                            format!("hatch_egg:{}", egg.egg_id),
                            Rect::new(490.0, y_egg + 48.0, 250.0, 44.0),
                            "CHOCAR NO SANTUARIO",
                        )
                        .with_colors(UiColor::BTN_PRIMARY, Color::WHITE)
                        .with_scale(1.5),
                    );
                } else {
                    modal.add_text(
                        format!("CHOCANDO... RESTAM {}s", egg.remaining_secs()),
                        150.0,
                        y_egg + 56.0,
                        1.5,
                        Color::WHITE,
                    );
                }

                // Botão de vender / liberar ovo
                modal.add_button(
                    UiButton::new(
                        format!("sell_egg:{}", egg.egg_id),
                        Rect::new(750.0, y_egg + 48.0, 180.0, 44.0),
                        "VENDER",
                    )
                    .with_colors(UiColor::BTN_SECONDARY, Color::WHITE)
                    .with_scale(1.5),
                );

                y_egg += 105.0;
            }
        }

        self.ui.open_modal(modal);
    }

    /// Abre janela modal do Bestiário (descobertas e resgate de marcos de recompensa).
    pub fn open_bestiary_modal(&mut self) {
        let mut modal = UiModal::new(
            "modal_bestiary",
            "COMPENDIO DO BESTIARIO",
            Rect::new(120.0, 180.0, 840.0, 820.0),
        );

        let discovered = self.world.bestiary.discovered_count();
        let total = self.catalog.count();
        let pct = self.world.bestiary.completion_percentage(&self.catalog);

        modal.add_text(
            format!("PROGRESSO DE COLECAO: {} / {} ESPECIES ({:.1}%)", discovered, total, pct),
            150.0,
            250.0,
            1.8,
            UiColor::GOLD_ACCENT,
        );
        modal.add_progress_bar(UiProgressBar::new(
            Rect::new(150.0, 285.0, 780.0, 20.0),
            (pct / 100.0).clamp(0.0, 1.0) as f32,
            UiColor::XP_FILL,
        ));

        modal.add_text(
            "MARCOS DE CONQUISTA & RECOMPENSAS:",
            150.0,
            325.0,
            1.7,
            Color::WHITE,
        );

        let mut y_m = 360.0;
        for milestone in self.milestones_catalog.all_sorted() {
            let is_claimed = self.world.bestiary.is_milestone_claimed(&milestone.id);
            let is_satisfied = self.world.bestiary.is_milestone_met(milestone, &self.catalog);

            modal.add_text(
                format!("MARCO: {} - {}", milestone.title.to_uppercase().replace("Ã", "A"), milestone.description),
                150.0,
                y_m,
                1.5,
                Color::WHITE,
            );
            modal.add_text(
                format!("PREMIO: +{} OURO, +{} COMIDA", milestone.reward.gold, milestone.reward.food),
                150.0,
                y_m + 25.0,
                1.4,
                UiColor::GOLD_FILL,
            );

            let (btn_label, btn_color, disabled) = if is_claimed {
                ("JA RESGATADO", UiColor::BTN_DISABLED, true)
            } else if is_satisfied {
                ("REIVINDICAR PREMIO!", UiColor::BTN_PRIMARY, false)
            } else {
                ("BLOQUEADO", UiColor::BTN_DISABLED, true)
            };

            modal.add_button(
                UiButton::new(
                    format!("claim_milestone:{}", milestone.id),
                    Rect::new(650.0, y_m + 8.0, 280.0, 44.0),
                    btn_label,
                )
                .with_colors(btn_color, Color::WHITE)
                .with_disabled(disabled)
                .with_scale(1.5),
            );

            y_m += 74.0;
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
            &format!(
                "BESTIGARD SANCTUARY - NVL {} (XP {}/{})",
                self.world.player_level,
                self.world.player_xp,
                self.world.player_xp_for_next_level()
            ),
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
                        "btn_shop_tab:buildings" => {
                            self.shop_tab = ShopTab::Buildings;
                            self.open_shop_modal();
                        }
                        "btn_shop_tab:eggs" => {
                            self.shop_tab = ShopTab::Eggs;
                            self.open_shop_modal();
                        }
                        id if id.starts_with("shop_buy_egg:") => {
                            let spec_id = &id["shop_buy_egg:".len()..];
                            match self.world.buy_egg(spec_id, &self.catalog) {
                                Ok(egg_id) => {
                                    println!("Loja: Ovo #{} da espécie '{}' comprado e depositado no ninho!", egg_id, spec_id);
                                }
                                Err(e) => println!("Loja: Falha ao comprar ovo '{}': {}", spec_id, e),
                            }
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
                        id if id.starts_with("creature_nav:") => {
                            if let Ok(cid) = id["creature_nav:".len()..].parse::<u64>() {
                                self.open_creature_modal(cid);
                            }
                        }
                        id if id.starts_with("relocate_creature:") => {
                            let parts: Vec<&str> = id["relocate_creature:".len()..].split(':').collect();
                            if parts.len() == 2 {
                                if let (Ok(cid), Ok(hid)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                                    match self.world.relocate_creature(cid, hid, &self.catalog, &self.habitats_catalog) {
                                        Ok(_) => println!("UI: Criatura #{} transferida para o Habitat #{} com sucesso!", cid, hid),
                                        Err(e) => println!("UI: Falha ao transferir criatura #{}: {}", cid, e),
                                    }
                                    self.open_creature_modal(cid);
                                }
                            }
                        }
                        "btn_open_breeding" => {
                            self.open_breeding_modal();
                        }
                        "btn_cycle_parent1" => {
                            let mut eligible: Vec<u64> = self.world.creatures.iter().filter(|(_, c)| c.can_breed()).map(|(&id, _)| id).collect();
                            eligible.sort();
                            if eligible.len() >= 2 {
                                let current = self.selected_parent1_id.unwrap_or(eligible[0]);
                                let curr_idx = eligible.iter().position(|&id| id == current).unwrap_or(0);
                                let mut next_idx = (curr_idx + 1) % eligible.len();
                                if Some(eligible[next_idx]) == self.selected_parent2_id {
                                    next_idx = (next_idx + 1) % eligible.len();
                                }
                                self.selected_parent1_id = Some(eligible[next_idx]);
                                self.open_breeding_modal();
                            }
                        }
                        "btn_cycle_parent2" => {
                            let mut eligible: Vec<u64> = self.world.creatures.iter().filter(|(_, c)| c.can_breed()).map(|(&id, _)| id).collect();
                            eligible.sort();
                            if eligible.len() >= 2 {
                                let current = self.selected_parent2_id.unwrap_or(eligible[1]);
                                let curr_idx = eligible.iter().position(|&id| id == current).unwrap_or(1);
                                let mut next_idx = (curr_idx + 1) % eligible.len();
                                if Some(eligible[next_idx]) == self.selected_parent1_id {
                                    next_idx = (next_idx + 1) % eligible.len();
                                }
                                self.selected_parent2_id = Some(eligible[next_idx]);
                                self.open_breeding_modal();
                            }
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
                            let mut eligible: Vec<u64> = self
                                .world
                                .creatures
                                .iter()
                                .filter(|(_, c)| c.can_breed())
                                .map(|(&id, _)| id)
                                .collect();
                            eligible.sort();
                            if eligible.len() >= 2 {
                                let p1 = self.selected_parent1_id.unwrap_or(eligible[0]);
                                let p2 = self.selected_parent2_id.unwrap_or(eligible[1]);
                                match self.world.breed_creatures(p1, p2, &self.catalog, 9999) {
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
                        id if id.starts_with("btn_upgrade_hab_action:") => {
                            if let Ok(hid) = id["btn_upgrade_hab_action:".len()..].parse::<u64>() {
                                match self.world.upgrade_habitat(hid, &self.habitats_catalog) {
                                    Ok(new_tier) => println!("UI: Habitat #{} melhorado com sucesso para o Tier {}!", hid, new_tier),
                                    Err(e) => println!("UI: Falha ao melhorar habitat #{}: {}", hid, e),
                                }
                                self.open_habitat_modal(hid);
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
                                    self.world.add_player_xp(35);
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
                        "btn_open_hatchery" => {
                            self.open_hatchery_modal();
                        }
                        "btn_open_bestiary" => {
                            self.open_bestiary_modal();
                        }
                        id if id.starts_with("plant_crop:") => {
                            let parts: Vec<&str> = id["plant_crop:".len()..].split(':').collect();
                            if parts.len() == 2 {
                                if let (Ok(farm_id), Some(crop)) = (parts[0].parse::<u64>(), self.crops_catalog.get(parts[1])) {
                                    match self.world.plant_crop(farm_id, crop) {
                                        Ok(_) => println!("Fazenda #{}: plantado '{}'.", farm_id, crop.name),
                                        Err(e) => println!("Fazenda #{}: falha ao plantar: {}", farm_id, e),
                                    }
                                    self.open_farm_modal(farm_id);
                                }
                            }
                        }
                        id if id.starts_with("harvest_farm:") => {
                            if let Ok(farm_id) = id["harvest_farm:".len()..].parse::<u64>() {
                                match self.world.harvest_farm(farm_id) {
                                    Ok(food) => println!("Fazenda #{}: colhido +{} comida!", farm_id, food),
                                    Err(e) => println!("Fazenda #{}: falha ao colher: {}", farm_id, e),
                                }
                                self.open_farm_modal(farm_id);
                            }
                        }
                        id if id.starts_with("hatch_egg:") => {
                            if let Ok(egg_id) = id["hatch_egg:".len()..].parse::<u64>() {
                                let target_hab = self.world.habitats.iter().find_map(|(&hid, h)| {
                                    let htype = self.habitats_catalog.get(&h.type_id)?;
                                    let tier_info = htype.get_tier_info(h.tier)?;
                                    if h.creature_ids.len() < tier_info.max_creatures {
                                        let egg = self.world.hatchery.eggs.iter().find(|e| e.egg_id == egg_id)?;
                                        let spec = self.catalog.get(&egg.species_id)?;
                                        if htype.accepts_elements(&spec.elements) {
                                            return Some(hid);
                                        }
                                    }
                                    None
                                });

                                if let Some(target_hid) = target_hab {
                                    match self.world.hatch_egg(egg_id, target_hid, &self.catalog, &self.habitats_catalog) {
                                        Ok(cid) => println!("Ninho: Ovo #{} chocado com sucesso -> Nova criatura #{} no Habitat #{}!", egg_id, cid, target_hid),
                                        Err(e) => println!("Ninho: Falha ao chocar ovo #{}: {}", egg_id, e),
                                    }
                                } else {
                                    println!("Ninho: Nenhum habitat compatível com vaga disponível para este ovo.");
                                }
                                self.open_hatchery_modal();
                            }
                        }
                        id if id.starts_with("sell_egg:") => {
                            if let Ok(egg_id) = id["sell_egg:".len()..].parse::<u64>() {
                                match self.world.sell_egg(egg_id, &self.catalog) {
                                    Ok(gold) => println!("Ninho: Ovo #{} vendido por +{} ouro!", egg_id, gold),
                                    Err(e) => println!("Ninho: Falha ao vender ovo #{}: {}", egg_id, e),
                                }
                                self.open_hatchery_modal();
                            }
                        }
                        id if id.starts_with("claim_milestone:") => {
                            let m_id = &id["claim_milestone:".len()..];
                            match self.world.claim_milestone(m_id, &self.milestones_catalog, &self.catalog) {
                                Ok(reward) => println!("Bestiário: Marco '{}' resgatado! +{} ouro, +{} comida.", m_id, reward.gold, reward.food),
                                Err(e) => println!("Bestiário: Erro ao resgatar marco '{}': {}", m_id, e),
                            }
                            self.open_bestiary_modal();
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

                // Se tocou sobre uma fazenda, abre modal da fazenda
                if let Some((&farm_id, _)) = self.world.farms.iter().find(|(_, f)| {
                    let fx = f.grid_x as i32;
                    let fy = f.grid_y as i32;
                    cell.0 >= fx && cell.0 < fx + f.width as i32 && cell.1 >= fy && cell.1 < fy + f.height as i32
                }) {
                    println!("UI: Selecionada Fazenda #{} no mapa -> Abrindo menu de fazenda", farm_id);
                    self.open_farm_modal(farm_id);
                    continue;
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
    println!("1. Testando toque no botão de HUD [COLETAR OURO] (pos: 400, 40)...");
    let mut hud_input = InputManager::new();
    hud_input.on_mouse_button_down(MouseButton::Left, 400.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 400.0, 40.0);
    state.on_input(&hud_input);
    hud_input.clear_events();

    // B) Toque no botão global do HUD: [CRIATURA] para abrir modal de inspeção
    println!("\n2. Abrindo diálogo modal de Gestão da Criatura Ignis [CRIATURA] (pos: 500, 40)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 500.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 500.0, 40.0);
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

    println!("Abrindo modal do [CRUZAMENTO] (pos: 600, 40)...");
    hud_input.on_mouse_button_down(MouseButton::Left, 600.0, 40.0);
    hud_input.on_mouse_button_up(MouseButton::Left, 600.0, 40.0);
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

    println!("1. Tocando em [LOJA] (pos: 890, 40) e escolhendo [HABITAT WATER]...");
    tap_at(state, 890.0, 40.0);
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
    tap_at(state, 890.0, 40.0);
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

    println!("1. Abrindo Arena Elemental de Batalha via botão [ARENA] no HUD (pos: 990, 40)...");
    tap_at(state, 990.0, 40.0);
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

    // 11. Demonstração dos Modais de Fazenda, Ninho e Bestiário (Passo 5.6)
    println!("\n--- MODAIS DE FAZENDA, NINHO E BESTIÁRIO (PASSO 5.6) ---");

    // A) Toque na Fazenda (célula 6, 2) abre FarmModal
    let (farm_sx, farm_sy) = cell_to_screen(state, 6, 2);
    println!("1. Tocando na Fazenda em (6, 2) (pos: {:.0}, {:.0})...", farm_sx, farm_sy);
    tap_at(state, farm_sx, farm_sy);
    assert!(state.ui.is_modal_open());
    assert_eq!(state.ui.active_modal.as_ref().unwrap().id, "modal_farm");
    println!("FarmModal aberto com sucesso!");

    // Planta Fruta Silvestre via botão da fazenda (primeira semente na lista)
    println!("2. Plantando Fruta Silvestre na Fazenda via botão interno...");
    tap_at(state, 300.0, 520.0);
    let farm_id = state.selected_farm_id.unwrap();
    assert!(matches!(state.world.farms[&farm_id].state, game_core::FarmState::Growing { .. }));
    println!("Plantação iniciada! Estado da fazenda: em crescimento.");
    state.ui.close_modal();

    // B) Botão [NINHO] no HUD (pos: 700, 40) abre HatcheryModal
    println!("3. Abrindo modal do Ninho de Incubação [NINHO] (pos: 700, 40)...");
    tap_at(state, 700.0, 40.0);
    assert!(state.ui.is_modal_open());
    assert_eq!(state.ui.active_modal.as_ref().unwrap().id, "modal_hatchery");
    println!("HatcheryModal aberto com sucesso! Ovos em incubação: {}", state.world.hatchery.eggs.len());
    state.ui.close_modal();

    // C) Botão [BESTIARIO] no HUD (pos: 800, 40) abre BestiaryModal
    println!("4. Abrindo modal do Bestiário [BESTIARIO] (pos: 800, 40)...");
    tap_at(state, 800.0, 40.0);
    assert!(state.ui.is_modal_open());
    assert_eq!(state.ui.active_modal.as_ref().unwrap().id, "modal_bestiary");
    println!("BestiaryModal aberto com sucesso! Progresso de coleção: {:.1}%", state.world.bestiary.completion_percentage(&state.catalog));
    state.ui.close_modal();
    assert!(!state.ui.is_modal_open());

    println!("\n>> Interface de Usuário (HUD e Menus Modais) validada com 100% de sucesso! <<");
    println!(">> Modo de Construção e Limpeza de Terreno validados com 100% de sucesso! <<");
    println!(">> Arena Elemental de Batalha e UI Interativa validadas com 100% de sucesso! <<");
    println!(">> Modais de Fazenda, Ninho e Bestiário validados com 100% de sucesso! <<");

    // 12. Demonstração e Verificação das Funcionalidades do Passo 5.7
    println!("\n--- SELEÇÃO COMPLETA, UPGRADE DE HABITAT E LOJA DE OVOS (PASSO 5.7) ---");

    // A) Navegação entre criaturas no CreatureModal
    println!("1. Abrindo CreatureModal e testando navegação cíclica (< ANTERIOR / PROXIMA >)...");
    let cids: Vec<u64> = {
        let mut ids: Vec<u64> = state.world.creatures.keys().copied().collect();
        ids.sort();
        ids
    };
    assert!(cids.len() >= 2);
    state.open_creature_modal(cids[0]);
    assert_eq!(state.selected_creature_id, Some(cids[0]));
    // Clica em [PROXIMA >] (x=800.0, y=310.0)
    tap_at(state, 800.0, 310.0);
    assert_eq!(state.selected_creature_id, Some(cids[1]));
    // Clica em [< ANTERIOR] (x=200.0, y=310.0)
    tap_at(state, 200.0, 310.0);
    assert_eq!(state.selected_creature_id, Some(cids[0]));
    println!("Navegação entre criaturas validada com sucesso!");

    // B) Realocação de Criatura: criatura do habitat fire (#10) ou water (#20)
    println!("2. Testando realocação de criatura para outro habitat compatível...");
    let c_to_move = state.world.creatures.values().find(|c| {
        let spec = state.catalog.get(&c.species_id).unwrap();
        state.world.habitats.iter().any(|(&hid, hab)| {
            Some(hid) != c.habitat_id && {
                let hab_type = state.habitats_catalog.get(&hab.type_id).unwrap();
                let tier_info = hab_type.get_tier_info(hab.tier).unwrap();
                hab.creature_ids.len() < tier_info.max_creatures && hab_type.accepts_elements(&spec.elements)
            }
        })
    }).map(|c| (c.instance_id, c.habitat_id));

    if let Some((cid, old_hid)) = c_to_move {
        state.open_creature_modal(cid);
        // Toca no botão de transferir (pos: 400.0, 565.0)
        tap_at(state, 400.0, 565.0);
        let new_hid = state.world.creatures[&cid].habitat_id;
        assert_ne!(new_hid, old_hid);
        println!("Criatura #{} realocada com sucesso do Habitat #{:?} para o Habitat #{:?}!", cid, old_hid, new_hid);
    }
    state.ui.close_modal();

    // C) Alternância manual de Pais no Altar de Cruzamento
    println!("3. Testando escolha e alternância manual de progenitores no Altar de Cruzamento...");
    tap_at(state, 556.0, 40.0); // Botão [CRUZAR] no HUD
    assert!(state.ui.is_modal_open());
    let _p1_initial = state.selected_parent1_id;
    // Clica em [TROCAR PAI 1] (pos: 750.0, 350.0)
    tap_at(state, 750.0, 350.0);
    println!("Progenitores selecionados após troca: Pai 1 = {:?}, Pai 2 = {:?}", state.selected_parent1_id, state.selected_parent2_id);
    assert_ne!(state.selected_parent1_id, state.selected_parent2_id);
    state.ui.close_modal();

    // D) Melhoria de Tier de Habitat no HabitatModal
    println!("4. Testando melhoria de Tier do Habitat #10...");
    let hab_id = 10;
    let initial_tier = state.world.habitats[&hab_id].tier;
    assert_eq!(initial_tier, 1);
    let up_cost = state.world.habitat_upgrade_cost(hab_id, &state.habitats_catalog).unwrap();
    if state.world.gold < up_cost {
        state.world.gold += up_cost;
    }
    state.open_habitat_modal(hab_id);
    // Toca no botão [MELHORAR HABITAT] (pos: 400.0, 510.0)
    tap_at(state, 400.0, 510.0);
    assert_eq!(state.world.habitats[&hab_id].tier, 2);
    let upgraded_hab_type = state.habitats_catalog.get(&state.world.habitats[&hab_id].type_id).unwrap();
    let tier2_info = upgraded_hab_type.get_tier_info(2).unwrap();
    assert_eq!(tier2_info.max_creatures, 3);
    assert_eq!(tier2_info.gold_capacity, 2500);
    println!("Habitat #{} promovido para Tier 2! Nova capacidade: {} criaturas, {} ouro max.", hab_id, tier2_info.max_creatures, tier2_info.gold_capacity);
    state.ui.close_modal();

    // E) Loja de Ovos Elementais
    println!("5. Testando compra de ovo elemental na Loja...");
    let eggs_before = state.world.hatchery.eggs.len();
    tap_at(state, 850.0, 40.0); // Botão [LOJA] no HUD
    assert!(state.ui.is_modal_open());
    // Clica na aba [OVOS ELEMENTAIS] (pos: 650.0, 350.0)
    tap_at(state, 650.0, 350.0);
    assert_eq!(state.shop_tab, ShopTab::Eggs);
    // Clica para comprar o primeiro ovo da lista (pos: 400.0, 450.0)
    state.world.gold += 500;
    tap_at(state, 400.0, 450.0);
    assert_eq!(state.world.hatchery.eggs.len(), eggs_before + 1);
    println!("Ovo elemental adquirido na loja! Ovos no ninho: {}", state.world.hatchery.eggs.len());
    state.ui.close_modal();

    println!("\n>> Seleção Completa, Upgrade de Habitat e Mercado de Ovos validados com 100% de sucesso! <<");
}
