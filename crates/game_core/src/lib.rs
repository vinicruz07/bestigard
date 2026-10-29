//! Game Core - Regras de negócio puras, modelo de criaturas, habitats e genética.
#![forbid(unsafe_code)]

pub mod bestiary;
pub mod breeding;
pub mod combat;
pub mod creature;
pub mod element;
pub mod farming;
pub mod habitat;
pub mod incubation;
pub mod obstacle;
pub mod save;
pub mod world;

pub use save::{SaveError, SaveGame, SAVE_VERSION};

pub use bestiary::{
    Bestiary, BestiaryError, CollectionMilestone, DiscoveredEntry, MilestoneCatalog,
    MilestoneCondition, MilestoneReward,
};
pub use breeding::{
    BreedingEngine, BreedingError, BreedingResult, DeterministicRng,
};
pub use combat::{
    BattleOutcome, BattleSession, CombatActionLog, CombatSkill, Combatant, Effectiveness,
    ElementalChart,
};
pub use creature::{
    CreatureInstance, CreatureSpecies, GrowthStage, Rarity, SpeciesCatalog,
};
pub use element::{Element, ElementId, ElementRegistry};
pub use farming::{
    CropCatalog, CropType, FarmError, FarmInstance, FarmState, FeedResult, FeedingError,
    FeedingSystem,
};
pub use habitat::{
    HabitatCatalog, HabitatError, HabitatInstance, HabitatTierInfo, HabitatType, IslandGrid,
};
pub use incubation::{
    EggInstance, HatcheryError, HatcheryInstance, IncubationSpeedModifier, ModifierSource,
};
pub use world::{SanctuaryWorld, WorldError, FARM_BUILD_COST_GOLD};
pub use obstacle::{Obstacle, ObstacleKind};

/// Retorna a versão da crate de regras centrais.
pub fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_version_is_valid() {
        assert!(!core_version().is_empty());
    }
}
