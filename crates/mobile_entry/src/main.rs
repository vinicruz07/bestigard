//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{App, EngineConfig, GameState};
use game_core::{
    CreatureInstance, ElementRegistry, SpeciesCatalog,
};

pub struct BestigardGame {
    elements: ElementRegistry,
    catalog: SpeciesCatalog,
    starter_creature: Option<CreatureInstance>,
    frames_rendered: u64,
    ticks_processed: u64,
}

impl BestigardGame {
    pub fn new() -> Self {
        Self {
            elements: ElementRegistry::with_default_elements(),
            catalog: SpeciesCatalog::with_starter_species(),
            starter_creature: None,
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
            "BestigardGame inicializado (Core v{}) - {} elementos, {} espécies base",
            game_core::core_version(),
            self.elements.count(),
            self.catalog.count()
        );

        // Cria a primeira criatura do jogador (Salamandra Ígnea Nv 1)
        if let Some(pyro) = self.catalog.get("pyro_salamander") {
            let mut creature = CreatureInstance::new(1, &pyro.id);
            creature.nickname = Some("Ignis".to_string());
            
            println!(
                "Criatura inicial concedida: '{}' ({}) - HP: {}, Ataque: {}, Renda: {} ouro/h",
                creature.nickname.as_deref().unwrap_or("Sem nome"),
                pyro.name,
                creature.calculate_hp(pyro),
                creature.calculate_attack(pyro),
                creature.calculate_gold_per_hour(pyro)
            );
            self.starter_creature = Some(creature);
        }
    }

    fn fixed_update(&mut self, _fixed_dt: f32) {
        self.ticks_processed += 1;
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

    // Simula 3 quadros de apresentação para validar o game loop
    for _ in 0..3 {
        app.step(1.0 / 60.0);
    }

    println!(
        "Game loop validado: {} ticks de simulação, {} quadros renderizados.",
        app.state().ticks_processed,
        app.state().frames_rendered
    );
}
