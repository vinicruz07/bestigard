//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{App, EngineConfig, GameState};
use game_core::{
    CreatureInstance, ElementRegistry, HabitatCatalog, HabitatInstance, IslandGrid, SpeciesCatalog,
};
use std::collections::HashMap;

pub struct BestigardGame {
    elements: ElementRegistry,
    catalog: SpeciesCatalog,
    habitats_catalog: HabitatCatalog,
    grid: IslandGrid,
    habitats: Vec<HabitatInstance>,
    creatures: HashMap<u64, CreatureInstance>,
    player_gold: u64,
    frames_rendered: u64,
    ticks_processed: u64,
}

impl BestigardGame {
    pub fn new() -> Self {
        Self {
            elements: ElementRegistry::with_default_elements(),
            catalog: SpeciesCatalog::with_starter_species(),
            habitats_catalog: HabitatCatalog::with_default_habitats(),
            grid: IslandGrid::new(16, 16),
            habitats: Vec::new(),
            creatures: HashMap::new(),
            player_gold: 250,
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
            "BestigardGame inicializado (Core v{}) - {} elementos, {} espécies, {} habitats",
            game_core::core_version(),
            self.elements.count(),
            self.catalog.count(),
            self.habitats_catalog.count()
        );

        // 1. Constrói o primeiro Habitat de Fogo na posição (2, 2) da ilha
        let fire_hab_type = self.habitats_catalog.get("fire_habitat").expect("Fire habitat should exist");
        let mut habitat = HabitatInstance::new(1, "fire_habitat", 2, 2);
        self.grid.place_habitat(&habitat, fire_hab_type).expect("Habitat placement should succeed");

        // 2. Cria a criatura inicial (Salamandra Ígnea) e aloca no habitat
        if let Some(pyro) = self.catalog.get("pyro_salamander") {
            let mut creature = CreatureInstance::new(1, &pyro.id);
            creature.nickname = Some("Ignis".to_string());
            creature.habitat_id = Some(habitat.instance_id);

            habitat.add_creature(&creature, fire_hab_type, &self.catalog).expect("Should add creature to habitat");
            self.creatures.insert(creature.instance_id, creature);

            println!(
                "Habitat #{} alocado em (2,2) com a criatura 'Ignis' ({}). Capacidade ouro: {}",
                habitat.instance_id,
                pyro.name,
                fire_hab_type.tiers[0].gold_capacity
            );
        }

        self.habitats.push(habitat);
    }

    fn fixed_update(&mut self, fixed_dt: f32) {
        self.ticks_processed += 1;

        // Atualiza a geração passiva de recursos dos habitats
        let fire_type = self.habitats_catalog.get("fire_habitat").unwrap();
        for hab in &mut self.habitats {
            // Acelera simulação no log de exemplo (1 tick = 10 segundos equivalentes)
            hab.generate_gold(fixed_dt as f64 * 10.0, &self.creatures, &self.catalog, fire_type);
        }
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

    // Simula alguns quadros de apresentação para demonstrar a geração de ouro
    for _ in 0..6 {
        app.step(1.0 / 60.0);
    }

    let hab = &mut app.state_mut().habitats[0];
    let collected = hab.collect_gold();
    app.state_mut().player_gold += collected;

    println!(
        "Game loop concluído: {} ticks, {} quadros. Ouro coletado do habitat: {}, Ouro total: {}",
        app.state().ticks_processed,
        app.state().frames_rendered,
        collected,
        app.state().player_gold
    );
}
