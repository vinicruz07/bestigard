//! Persistência segura em disco, resolução de caminhos multiplataforma e auto-save para desktop e mobile.
#![forbid(unsafe_code)]

use game_core::{
    HabitatCatalog, OfflineProgressReport, SanctuaryWorld, SaveError, SaveGame, SpeciesCatalog,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Resolvedor de caminhos de arquivos seguro por plataforma (Android vs Desktop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformPaths {
    base_dir: PathBuf,
}

impl PlatformPaths {
    pub const DEFAULT_APP_NAME: &'static str = "bestigard";
    pub const SAVE_FILENAME: &'static str = "savegame.json";
    pub const BACKUP_FILENAME: &'static str = "savegame.bak";
    pub const TEMP_FILENAME: &'static str = "savegame.tmp";

    /// Resolve o diretório padrão para a plataforma de execução atual.
    pub fn for_current_platform() -> Self {
        #[cfg(target_os = "android")]
        {
            // No Android, dados graváveis DEVEM ficar no armazenamento interno do app
            let base = PathBuf::from("/data/data/com.bestigard.game/files");
            Self::new(base)
        }
        #[cfg(not(target_os = "android"))]
        {
            let base = Self::resolve_desktop_dir().unwrap_or_else(|| PathBuf::from("save"));
            Self::new(base)
        }
    }

    /// Cria uma nova instância com diretório base customizado.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Garante que o diretório base existe no sistema de arquivos.
    pub fn ensure_base_dir(&mut self) -> std::io::Result<()> {
        if !self.base_dir.exists() {
            if let Err(err) = std::fs::create_dir_all(&self.base_dir) {
                // Fallback seguro caso o sistema restrinja pastas de sistema (ex: sandbox)
                let local = PathBuf::from("save");
                let _ = std::fs::create_dir_all(&local);
                self.base_dir = local;
                if !self.base_dir.exists() {
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    #[inline]
    pub fn save_file(&self) -> PathBuf {
        self.base_dir.join(Self::SAVE_FILENAME)
    }

    #[inline]
    pub fn backup_file(&self) -> PathBuf {
        self.base_dir.join(Self::BACKUP_FILENAME)
    }

    #[inline]
    pub fn temp_file(&self) -> PathBuf {
        self.base_dir.join(Self::TEMP_FILENAME)
    }

    fn resolve_desktop_dir() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
                let candidate = PathBuf::from(appdata).join(Self::DEFAULT_APP_NAME);
                if std::fs::create_dir_all(&candidate).is_ok() {
                    return Some(candidate);
                }
            }
            if let Ok(appdata) = std::env::var("APPDATA") {
                let candidate = PathBuf::from(appdata).join(Self::DEFAULT_APP_NAME);
                if std::fs::create_dir_all(&candidate).is_ok() {
                    return Some(candidate);
                }
            }
        }
        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                let candidate = PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join(Self::DEFAULT_APP_NAME);
                if std::fs::create_dir_all(&candidate).is_ok() {
                    return Some(candidate);
                }
            }
        }
        let fallback = PathBuf::from("save");
        let _ = std::fs::create_dir_all(&fallback);
        Some(fallback)
    }
}

/// Gerenciador de persistência atômica, backup, auto-save e recuperação de progresso offline.
pub struct PersistenceManager {
    paths: PlatformPaths,
    auto_save_interval_secs: f32,
    auto_save_accumulator: f32,
    last_saved_epoch_secs: u64,
    save_counter: u64,
    enabled: bool,
}

impl PersistenceManager {
    pub const DEFAULT_AUTO_SAVE_INTERVAL_SECS: f32 = 60.0;

    pub fn new(paths: PlatformPaths) -> Self {
        Self {
            paths,
            auto_save_interval_secs: Self::DEFAULT_AUTO_SAVE_INTERVAL_SECS,
            auto_save_accumulator: 0.0,
            last_saved_epoch_secs: 0,
            save_counter: 0,
            enabled: true,
        }
    }

    pub fn with_auto_save_interval(mut self, interval_secs: f32) -> Self {
        self.auto_save_interval_secs = interval_secs.max(1.0);
        self
    }

    #[inline]
    pub fn paths(&self) -> &PlatformPaths {
        &self.paths
    }

    #[inline]
    pub fn last_saved_epoch_secs(&self) -> u64 {
        self.last_saved_epoch_secs
    }

    #[inline]
    pub fn save_counter(&self) -> u64 {
        self.save_counter
    }

    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Retorna o timestamp Unix atual em segundos.
    pub fn current_epoch_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Salva o estado do mundo de forma atômica no disco:
    /// 1. Escreve os dados no arquivo temporário `.tmp`;
    /// 2. Cria cópia de segurança `.bak` do savegame anterior se ele existir;
    /// 3. Renomeia `.tmp` para `.json` atômico;
    /// 4. Atualiza timestamp e contadores.
    pub fn save_atomic(&mut self, world: &SanctuaryWorld) -> Result<PathBuf, SaveError> {
        self.paths
            .ensure_base_dir()
            .map_err(|e| SaveError::Serialize(format!("Não foi possível criar pasta de save: {}", e)))?;

        let timestamp = Self::current_epoch_seconds();
        let temp_path = self.paths.temp_file();
        let target_path = self.paths.save_file();
        let backup_path = self.paths.backup_file();

        // 1. Grava no temporário
        SaveGame::save_to_file_with_timestamp(world, &temp_path, timestamp)?;

        // 2. Se já existe um save anterior válido, faz cópia para backup
        if target_path.exists() {
            let _ = std::fs::copy(&target_path, &backup_path);
        }

        // 3. Renomeação atômica do temporário para o arquivo oficial
        std::fs::rename(&temp_path, &target_path)
            .map_err(|e| SaveError::Serialize(format!("Falha na renomeação atômica de save: {}", e)))?;

        self.last_saved_epoch_secs = timestamp;
        self.save_counter += 1;
        self.auto_save_accumulator = 0.0;

        Ok(target_path)
    }

    /// Tenta carregar o arquivo principal `.json`; caso ausente ou corrompido, tenta recuperar via `.bak`.
    pub fn load_with_backup_recovery(&self) -> Result<(SanctuaryWorld, u64, bool), SaveError> {
        let save_path = self.paths.save_file();
        let backup_path = self.paths.backup_file();

        // 1. Tenta carregar o arquivo principal
        if save_path.exists() {
            match SaveGame::load_from_file_with_timestamp(&save_path) {
                Ok((world, ts)) => return Ok((world, ts, false)),
                Err(err) => {
                    eprintln!(
                        "Aviso: Savegame principal corrompido ou ilegível ({}). Tentando recuperar backup (.bak)...",
                        err
                    );
                }
            }
        }

        // 2. Tenta recuperar do backup (.bak)
        if backup_path.exists() {
            let (recovered_world, ts) = SaveGame::load_from_file_with_timestamp(&backup_path)?;
            eprintln!("Recuperação de backup (.bak) concluída com sucesso!");
            return Ok((recovered_world, ts, true));
        }

        Err(SaveError::Parse("Nenhum savegame ou backup encontrado".into()))
    }

    /// Carrega o savegame existente aplicando progresso offline, ou invoca a factory inicial se não houver save prévio.
    pub fn load_or_create<F>(
        &self,
        starter_factory: F,
        species_catalog: &SpeciesCatalog,
        habitats_catalog: &HabitatCatalog,
    ) -> (SanctuaryWorld, Option<OfflineProgressReport>)
    where
        F: FnOnce() -> SanctuaryWorld,
    {
        match self.load_with_backup_recovery() {
            Ok((mut world, saved_ts, _recovered_from_backup)) => {
                let now = Self::current_epoch_seconds();
                let report = if saved_ts > 0 && now > saved_ts {
                    let elapsed_secs = (now - saved_ts) as f64;
                    Some(world.apply_offline_progress(elapsed_secs, species_catalog, habitats_catalog))
                } else {
                    None
                };
                (world, report)
            }
            Err(_) => (starter_factory(), None),
        }
    }

    /// Avança o cronômetro de auto-save e executa a gravação periódica se o intervalo for atingido.
    pub fn update_auto_save(
        &mut self,
        dt_secs: f32,
        world: &SanctuaryWorld,
    ) -> Option<Result<PathBuf, SaveError>> {
        if !self.enabled || dt_secs <= 0.0 {
            return None;
        }

        self.auto_save_accumulator += dt_secs;
        if self.auto_save_accumulator >= self.auto_save_interval_secs {
            Some(self.save_atomic(world))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{HabitatCatalog, SpeciesCatalog};

    fn make_test_world() -> SanctuaryWorld {
        let species = SpeciesCatalog::with_starter_species();
        let habitats = HabitatCatalog::with_default_habitats();
        SanctuaryWorld::with_starter_setup("Tester", &species, &habitats).unwrap()
    }

    #[test]
    fn test_platform_paths_resolution() {
        let custom_dir = std::env::temp_dir().join("bestigard_paths_test");
        let paths = PlatformPaths::new(&custom_dir);
        assert_eq!(paths.base_dir(), custom_dir.as_path());
        assert_eq!(paths.save_file(), custom_dir.join("savegame.json"));
        assert_eq!(paths.backup_file(), custom_dir.join("savegame.bak"));
        assert_eq!(paths.temp_file(), custom_dir.join("savegame.tmp"));

        let platform_paths = PlatformPaths::for_current_platform();
        assert!(!platform_paths.base_dir().as_os_str().is_empty());
    }

    #[test]
    fn test_atomic_save_and_backup_creation() {
        let test_dir = std::env::temp_dir().join(format!("bestigard_save_test_{}", platform_manager_test_nonce()));
        let paths = PlatformPaths::new(&test_dir);
        let mut manager = PersistenceManager::new(paths);

        let mut world = make_test_world();
        world.gold = 777;

        // Primeiro save
        let res = manager.save_atomic(&world);
        assert!(res.is_ok());
        assert!(manager.paths().save_file().exists());
        assert!(!manager.paths().backup_file().exists()); // Backup só é criado no segundo save
        assert_eq!(manager.save_counter(), 1);

        // Segundo save (modificado) -> deve gerar o .bak com o valor anterior (777)
        world.gold = 999;
        let res2 = manager.save_atomic(&world);
        assert!(res2.is_ok());
        assert!(manager.paths().backup_file().exists());
        assert_eq!(manager.save_counter(), 2);

        // Carrega o oficial
        let (loaded, _, recovered) = manager.load_with_backup_recovery().unwrap();
        assert_eq!(loaded.gold, 999);
        assert!(!recovered);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_recovery_from_corrupted_primary_via_backup() {
        let test_dir = std::env::temp_dir().join(format!("bestigard_recovery_test_{}", platform_manager_test_nonce()));
        let paths = PlatformPaths::new(&test_dir);
        let mut manager = PersistenceManager::new(paths);

        let mut world = make_test_world();
        world.gold = 1234;

        // 1. Salva estado inicial
        manager.save_atomic(&world).unwrap();

        // 2. Salva novo estado para gerar backup
        world.gold = 5678;
        manager.save_atomic(&world).unwrap();

        // 3. Corrompe o arquivo principal .json
        std::fs::write(manager.paths().save_file(), b"GARBAGE_DATA_CORRUPT").unwrap();

        // 4. Carrega: deve detectar corrupção no principal e recuperar com sucesso do .bak
        let (recovered_world, _, was_recovered) = manager.load_with_backup_recovery().unwrap();
        assert!(was_recovered);
        assert_eq!(recovered_world.gold, 1234);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_auto_save_timer_trigger() {
        let test_dir = std::env::temp_dir().join(format!("bestigard_autosave_test_{}", platform_manager_test_nonce()));
        let paths = PlatformPaths::new(&test_dir);
        let mut manager = PersistenceManager::new(paths).with_auto_save_interval(10.0);
        let world = make_test_world();

        // Passos curtos não disparam save
        assert!(manager.update_auto_save(3.0, &world).is_none());
        assert!(manager.update_auto_save(4.0, &world).is_none());
        assert_eq!(manager.save_counter(), 0);

        // Ao ultrapassar o intervalo de 10s -> dispara o save
        let triggered = manager.update_auto_save(3.5, &world);
        assert!(triggered.is_some());
        assert!(triggered.unwrap().is_ok());
        assert_eq!(manager.save_counter(), 1);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    fn platform_manager_test_nonce() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}
