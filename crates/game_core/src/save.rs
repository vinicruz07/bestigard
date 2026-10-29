//! Serialização robusta do savegame (JSON versionado) com validação de integridade.
#![forbid(unsafe_code)]

use crate::world::SanctuaryWorld;
use serde::{Deserialize, Serialize};

/// Versão atual do formato de savegame.
pub const SAVE_VERSION: u32 = 1;

/// Marcador de formato para rejeitar arquivos que não são savegames do Bestigard.
pub const SAVE_MAGIC: &str = "BESTIGARD_SAVE";

/// Erros de leitura/escrita de savegame.
#[derive(Debug)]
pub enum SaveError {
    Serialize(String),
    Parse(String),
    InvalidMagic(String),
    UnsupportedVersion { found: u32, supported: u32 },
    Corrupted(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(e) => write!(f, "Falha ao serializar savegame: {}", e),
            Self::Parse(e) => write!(f, "Savegame ilegível: {}", e),
            Self::InvalidMagic(m) => write!(f, "Formato de arquivo inválido: '{}'", m),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "Versão de savegame {} não suportada (máxima suportada: {})",
                found, supported
            ),
            Self::Corrupted(e) => write!(f, "Savegame corrompido: {}", e),
        }
    }
}

impl std::error::Error for SaveError {}

/// Envelope versionado que guarda o estado completo do mundo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    pub magic: String,
    pub version: u32,
    pub world: SanctuaryWorld,
}

/// Cabeçalho mínimo, lido antes do corpo para validar versão sem depender do schema.
#[derive(Deserialize)]
struct SaveHeader {
    magic: String,
    version: u32,
}

impl SaveGame {
    pub fn new(world: SanctuaryWorld) -> Self {
        Self { magic: SAVE_MAGIC.to_string(), version: SAVE_VERSION, world }
    }

    /// Serializa o mundo em JSON versionado.
    pub fn to_json(world: &SanctuaryWorld) -> Result<String, SaveError> {
        serde_json::to_string_pretty(&Self::new(world.clone()))
            .map_err(|e| SaveError::Serialize(e.to_string()))
    }

    /// Desserializa e valida magic, versão e consistência referencial.
    pub fn from_json(json: &str) -> Result<SanctuaryWorld, SaveError> {
        let header: SaveHeader =
            serde_json::from_str(json).map_err(|e| SaveError::Parse(e.to_string()))?;
        if header.magic != SAVE_MAGIC {
            return Err(SaveError::InvalidMagic(header.magic));
        }
        if header.version == 0 || header.version > SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion {
                found: header.version,
                supported: SAVE_VERSION,
            });
        }
        let save: SaveGame =
            serde_json::from_str(json).map_err(|e| SaveError::Parse(e.to_string()))?;
        validate_world(&save.world)?;
        Ok(save.world)
    }

    /// Salva em arquivo de forma atômica (escreve em `.tmp` e renomeia).
    pub fn save_to_file(world: &SanctuaryWorld, path: &std::path::Path) -> Result<(), SaveError> {
        let json = Self::to_json(world)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json).map_err(|e| SaveError::Serialize(e.to_string()))?;
        std::fs::rename(&tmp, path).map_err(|e| SaveError::Serialize(e.to_string()))
    }

    /// Carrega e valida um savegame de arquivo.
    pub fn load_from_file(path: &std::path::Path) -> Result<SanctuaryWorld, SaveError> {
        let json = std::fs::read_to_string(path).map_err(|e| SaveError::Parse(e.to_string()))?;
        Self::from_json(&json)
    }
}

/// Verifica invariantes básicas do mundo carregado.
fn validate_world(world: &SanctuaryWorld) -> Result<(), SaveError> {
    if !world.total_simulated_seconds.is_finite() || world.total_simulated_seconds < 0.0 {
        return Err(SaveError::Corrupted("tempo simulado inválido".into()));
    }
    for (key, c) in &world.creatures {
        if *key >= world.next_id_counter {
            return Err(SaveError::Corrupted(format!("id #{} >= contador de ids", key)));
        }
        if let Some(h) = c.habitat_id {
            if !world.habitats.contains_key(&h) {
                return Err(SaveError::Corrupted(format!(
                    "criatura #{} referencia habitat #{} inexistente",
                    key, h
                )));
            }
        }
    }
    for key in world.habitats.keys() {
        if *key >= world.next_id_counter {
            return Err(SaveError::Corrupted(format!("id #{} >= contador de ids", key)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::SpeciesCatalog;
    use crate::habitat::HabitatCatalog;

    fn starter() -> SanctuaryWorld {
        SanctuaryWorld::with_starter_setup(
            "Tester",
            &SpeciesCatalog::with_starter_species(),
            &HabitatCatalog::with_default_habitats(),
        )
        .expect("starter")
    }

    #[test]
    fn test_roundtrip_preserves_state() {
        let w = starter();
        let json = SaveGame::to_json(&w).unwrap();
        let back = SaveGame::from_json(&json).unwrap();
        assert_eq!(back.player_name, w.player_name);
        assert_eq!(back.gold, w.gold);
        assert_eq!(back.creatures.len(), w.creatures.len());
        assert_eq!(back.habitats.len(), w.habitats.len());
        assert_eq!(back.next_id_counter, w.next_id_counter);
    }

    #[test]
    fn test_rejects_future_version() {
        let w = starter();
        let json = SaveGame::to_json(&w).unwrap().replacen(
            &format!("\"version\": {}", SAVE_VERSION),
            "\"version\": 999",
            1,
        );
        assert!(matches!(
            SaveGame::from_json(&json),
            Err(SaveError::UnsupportedVersion { found: 999, .. })
        ));
    }

    #[test]
    fn test_rejects_bad_magic_and_garbage() {
        let w = starter();
        let json = SaveGame::to_json(&w).unwrap().replace(SAVE_MAGIC, "OTHER");
        assert!(matches!(SaveGame::from_json(&json), Err(SaveError::InvalidMagic(_))));
        assert!(matches!(SaveGame::from_json("not json"), Err(SaveError::Parse(_))));
    }

    #[test]
    fn test_rejects_dangling_habitat_reference() {
        let mut w = starter();
        w.habitats.clear();
        let json = SaveGame::to_json(&w).unwrap();
        assert!(matches!(SaveGame::from_json(&json), Err(SaveError::Corrupted(_))));
    }

    #[test]
    fn test_file_roundtrip() {
        let w = starter();
        let path = std::env::temp_dir().join("bestigard_test_save.json");
        SaveGame::save_to_file(&w, &path).unwrap();
        let back = SaveGame::load_from_file(&path).unwrap();
        assert_eq!(back.gold, w.gold);
        let _ = std::fs::remove_file(&path);
    }
}
