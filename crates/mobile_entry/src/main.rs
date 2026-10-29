//! Mobile / Desktop Runner para Bestigard.
#![forbid(unsafe_code)]

use engine::{App, EngineConfig, GameState};
use game_core::{
    CropCatalog, CreatureInstance, ElementRegistry, FarmInstance, FeedingSystem, HabitatCatalog,
    HabitatInstance, IslandGrid, SpeciesCatalog,
};
use std::collections::HashMap;

pub struct BestigardGame {
    elements: ElementRegistry,
    catalog: SpeciesCatalog,
    habitats_catalog: HabitatCatalog,
    crops_catalog: CropCatalog,
    grid: IslandGrid,
    habitats: Vec<HabitatInstance>,
    farms: Vec<FarmInstance>,
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
            grid: IslandGrid::new(16, 16),
            habitats: Vec::new(),
            farms: Vec::new(),
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
            "BestigardGame inicializado (Core v{}) - {} elementos, {} espécies, {} culturas",
            game_core::core_version(),
            self.elements.count(),
            self.catalog.count(),
            self.crops_catalog.count()
        );

        // 1. Constrói o Habitat de Fogo na posição (2, 2)
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
        }
        self.habitats.push(habitat);

        // 3. Constrói uma Fazenda de Comida na posição (6, 2) e planta Fruta Silvestre
        let mut farm = FarmInstance::new(100, 6, 2);
        let berry = self.crops_catalog.get("wild_berry").expect("Wild berry crop should exist");
        farm.plant_crop(berry, &mut self.player_gold).expect("Crop planting should succeed");
        println!(
            "Fazenda #{} plantou '{}' por {} ouro. Saldo de ouro restante: {}",
            farm.instance_id, berry.name, berry.gold_cost, self.player_gold
        );
        self.farms.push(farm);
    }

    fn fixed_update(&mut self, fixed_dt: f32) {
        self.ticks_processed += 1;

        // Atualiza a produção de ouro dos habitats
        let fire_type = self.habitats_catalog.get("fire_habitat").unwrap();
        for hab in &mut self.habitats {
            hab.generate_gold(fixed_dt as f64 * 60.0, &self.creatures, &self.catalog, fire_type);
        }

        // Atualiza o crescimento das plantações nas fazendas
        for farm in &mut self.farms {
            farm.update_growth(fixed_dt as f64 * 30.0);
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

    // Executa 40 passos para simular o tempo de colheita (15s necessários)
    for _ in 0..40 {
        app.step(1.0 / 60.0);
    }


    // Colhe a comida da fazenda
    let harvested_food = app.state_mut().farms[0].harvest().expect("Harvest should succeed");
    app.state_mut().player_food += harvested_food;
    println!("Colheita concluída! +{} de comida adicionada ao armazém.", harvested_food);

    // Alimenta a criatura Ignis
    let state = app.state_mut();
    if let Some(creature) = state.creatures.get_mut(&1) {
        let feed_res = FeedingSystem::feed_creature(creature, &mut state.player_food).expect("Feeding should succeed");
        println!(
            "Criatura '{}' alimentada (gasto: {} comida). Nível anterior: {}, Novo nível: {}, Evoluiu: {}",
            creature.nickname.as_deref().unwrap_or(""),
            feed_res.food_spent,
            feed_res.previous_level,
            feed_res.new_level,
            feed_res.evolved
        );
    }
}
