//! Game Core - Regras de negócio puras, modelo de criaturas, habitats e genética.
#![forbid(unsafe_code)]

pub mod creature;
pub mod element;

pub use creature::{
    CreatureInstance, CreatureSpecies, GrowthStage, Rarity, SpeciesCatalog,
};
pub use element::{Element, ElementId, ElementRegistry};

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
