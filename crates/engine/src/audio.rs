//! Subsistema de áudio agnóstico a plataforma da engine.
//!
//! Gerencia eventos de efeitos sonoros (SFX), controle de trilhas de fundo (BGM),
//! mixagem de volumes, muting independente e fila de eventos para backends nativos.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundCue {
    /// Clique genérico em botões de UI e menus.
    UiClick,
    /// Coleta de ouro de um ou mais habitats.
    GoldCollect,
    /// Colheita de culturas agrícolas em fazendas.
    FoodHarvest,
    /// Alimentação de uma criatura.
    FeedCreature,
    /// Evolução de estágio ou ganho de nível especial de criatura.
    CreatureEvolve,
    /// Novo ovo gerado no altar de cruzamento ou comprado na loja.
    EggDeposit,
    /// Eclosão de ovo e nascimento da criatura no santuário.
    EggHatch,
    /// Finalização de construção de habitat ou prédio na ilha.
    BuildingPlaced,
    /// Remoção e limpeza de obstáculo do terreno.
    ObstacleCleared,
    /// Ataque padrão deferido em combate na arena.
    CombatAttackHit,
    /// Ataque super-efetivo elemental com dano amplificado.
    CombatSuperEffective,
    /// Vitória triunfante em duelo na arena.
    CombatVictory,
    /// Derrota de equipe na arena.
    CombatDefeat,
    /// Reivindicação de marco ou conquista no bestiário.
    MilestoneClaim,
}

impl SoundCue {
    pub fn name(&self) -> &'static str {
        match self {
            Self::UiClick => "ui_click",
            Self::GoldCollect => "gold_collect",
            Self::FoodHarvest => "food_harvest",
            Self::FeedCreature => "feed_creature",
            Self::CreatureEvolve => "creature_evolve",
            Self::EggDeposit => "egg_deposit",
            Self::EggHatch => "egg_hatch",
            Self::BuildingPlaced => "building_placed",
            Self::ObstacleCleared => "obstacle_cleared",
            Self::CombatAttackHit => "combat_attack_hit",
            Self::CombatSuperEffective => "combat_super_effective",
            Self::CombatVictory => "combat_victory",
            Self::CombatDefeat => "combat_defeat",
            Self::MilestoneClaim => "milestone_claim",
        }
    }
}

/// Trilhas sonoras de fundo (BGM).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicTrack {
    /// Tema ambiente e sereno do santuário da ilha.
    SanctuaryTheme,
    /// Tema épico e agitado de batalha elemental.
    ArenaBattleTheme,
    /// Fanfarra curta de vitória triunfante.
    VictoryFanfare,
}

impl MusicTrack {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SanctuaryTheme => "bgm_sanctuary_theme",
            Self::ArenaBattleTheme => "bgm_arena_battle_theme",
            Self::VictoryFanfare => "bgm_victory_fanfare",
        }
    }
}

/// Estado da reprodução de música de fundo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MusicPlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Evento atômico de áudio gerado para consumo pelo backend nativo da plataforma.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioEvent {
    /// Reproduzir um efeito sonoro pontual com volume e afinação calculados.
    PlaySound {
        cue: SoundCue,
        volume: f32,
        pitch: f32,
    },
    /// Iniciar ou trocar a música de fundo atual.
    PlayMusic {
        track: MusicTrack,
        loop_playback: bool,
        volume: f32,
    },
    /// Pausar a música de fundo ativa.
    PauseMusic,
    /// Retomar a música pausada.
    ResumeMusic,
    /// Parar a reprodução da música de fundo.
    StopMusic,
}

/// Controlador e mixer agnóstico de áudio da engine.
#[derive(Debug, Clone)]
pub struct AudioController {
    master_volume: f32,
    sfx_volume: f32,
    bgm_volume: f32,
    is_master_muted: bool,
    is_sfx_muted: bool,
    is_bgm_muted: bool,
    current_music: Option<MusicTrack>,
    music_state: MusicPlaybackState,
    music_looping: bool,
    event_queue: Vec<AudioEvent>,
}

impl Default for AudioController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioController {
    /// Cria um novo controlador de áudio com volumes padrão (1.0) e sem mutes.
    pub fn new() -> Self {
        Self {
            master_volume: 1.0,
            sfx_volume: 1.0,
            bgm_volume: 0.8,
            is_master_muted: false,
            is_sfx_muted: false,
            is_bgm_muted: false,
            current_music: None,
            music_state: MusicPlaybackState::Stopped,
            music_looping: true,
            event_queue: Vec::new(),
        }
    }

    /// Define o volume master geral (clamp entre 0.0 e 1.0).
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Define o volume dos efeitos sonoros (SFX) (clamp entre 0.0 e 1.0).
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    /// Define o volume da música de fundo (BGM) (clamp entre 0.0 e 1.0).
    pub fn set_bgm_volume(&mut self, volume: f32) {
        self.bgm_volume = volume.clamp(0.0, 1.0);
    }

    /// Alterna o mute master global.
    pub fn set_master_muted(&mut self, muted: bool) {
        self.is_master_muted = muted;
    }

    /// Alterna o mute apenas para efeitos sonoros (SFX).
    pub fn set_sfx_muted(&mut self, muted: bool) {
        self.is_sfx_muted = muted;
    }

    /// Alterna o mute apenas para a música de fundo (BGM).
    pub fn set_bgm_muted(&mut self, muted: bool) {
        self.is_bgm_muted = muted;
    }

    /// Retorna o volume efetivo atual de SFX (levando em conta mute e master volume).
    pub fn effective_sfx_volume(&self) -> f32 {
        if self.is_master_muted || self.is_sfx_muted {
            0.0
        } else {
            self.master_volume * self.sfx_volume
        }
    }

    /// Retorna o volume efetivo atual de BGM (levando em conta mute e master volume).
    pub fn effective_bgm_volume(&self) -> f32 {
        if self.is_master_muted || self.is_bgm_muted {
            0.0
        } else {
            self.master_volume * self.bgm_volume
        }
    }

    /// Enfileira a reprodução de um efeito sonoro com afinação padrão (1.0).
    pub fn play_sound(&mut self, cue: SoundCue) {
        self.play_sound_with_pitch(cue, 1.0);
    }

    /// Enfileira a reprodução de um efeito sonoro com afinação customizada.
    pub fn play_sound_with_pitch(&mut self, cue: SoundCue, pitch: f32) {
        let volume = self.effective_sfx_volume();
        self.event_queue.push(AudioEvent::PlaySound {
            cue,
            volume,
            pitch: pitch.clamp(0.1, 4.0),
        });
    }

    /// Inicia a reprodução de uma trilha de música de fundo.
    pub fn play_music(&mut self, track: MusicTrack, loop_playback: bool) {
        // Evita reiniciar a mesma música caso já esteja tocando
        if self.current_music == Some(track) && self.music_state == MusicPlaybackState::Playing {
            return;
        }

        self.current_music = Some(track);
        self.music_state = MusicPlaybackState::Playing;
        self.music_looping = loop_playback;

        let volume = self.effective_bgm_volume();
        self.event_queue.push(AudioEvent::PlayMusic {
            track,
            loop_playback,
            volume,
        });
    }

    /// Pausa a música de fundo atual se estiver tocando.
    pub fn pause_music(&mut self) {
        if self.music_state == MusicPlaybackState::Playing {
            self.music_state = MusicPlaybackState::Paused;
            self.event_queue.push(AudioEvent::PauseMusic);
        }
    }

    /// Retoma a música de fundo pausada.
    pub fn resume_music(&mut self) {
        if self.music_state == MusicPlaybackState::Paused {
            self.music_state = MusicPlaybackState::Playing;
            self.event_queue.push(AudioEvent::ResumeMusic);
        }
    }

    /// Para a música de fundo atual.
    pub fn stop_music(&mut self) {
        if self.music_state != MusicPlaybackState::Stopped {
            self.music_state = MusicPlaybackState::Stopped;
            self.current_music = None;
            self.event_queue.push(AudioEvent::StopMusic);
        }
    }

    /// Retorna a trilha de música atualmente selecionada, se houver.
    pub fn current_music(&self) -> Option<MusicTrack> {
        self.current_music
    }

    /// Retorna o estado atual da música de fundo.
    pub fn music_state(&self) -> MusicPlaybackState {
        self.music_state
    }

    /// Retorna se a música atual foi configurada para repetição contínua.
    pub fn is_music_looping(&self) -> bool {
        self.music_looping
    }

    /// Retorna se há eventos pendentes na fila.
    pub fn has_pending_events(&self) -> bool {
        !self.event_queue.is_empty()
    }

    /// Drena todos os eventos de áudio pendentes acumulados até o frame atual.
    pub fn drain_events(&mut self) -> Vec<AudioEvent> {
        self.event_queue.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_volumes_and_muting() {
        let mut audio = AudioController::new();
        assert_eq!(audio.effective_sfx_volume(), 1.0);
        assert!((audio.effective_bgm_volume() - 0.8).abs() < f32::EPSILON);

        // Teste de clamping de volume
        audio.set_master_volume(1.5);
        assert_eq!(audio.master_volume, 1.0);
        audio.set_sfx_volume(-0.5);
        assert_eq!(audio.sfx_volume, 0.0);
        audio.set_sfx_volume(0.5);
        assert!((audio.effective_sfx_volume() - 0.5).abs() < f32::EPSILON);

        // Mute SFX
        audio.set_sfx_muted(true);
        assert_eq!(audio.effective_sfx_volume(), 0.0);
        assert!((audio.effective_bgm_volume() - 0.8).abs() < f32::EPSILON);

        // Mute Master zera tudo
        audio.set_sfx_muted(false);
        audio.set_master_muted(true);
        assert_eq!(audio.effective_sfx_volume(), 0.0);
        assert_eq!(audio.effective_bgm_volume(), 0.0);
    }

    #[test]
    fn test_sfx_event_generation() {
        let mut audio = AudioController::new();
        assert!(!audio.has_pending_events());

        audio.play_sound(SoundCue::GoldCollect);
        audio.play_sound_with_pitch(SoundCue::FeedCreature, 1.25);

        assert!(audio.has_pending_events());
        let events = audio.drain_events();
        assert_eq!(events.len(), 2);

        assert_eq!(
            events[0],
            AudioEvent::PlaySound {
                cue: SoundCue::GoldCollect,
                volume: 1.0,
                pitch: 1.0,
            }
        );
        assert_eq!(
            events[1],
            AudioEvent::PlaySound {
                cue: SoundCue::FeedCreature,
                volume: 1.0,
                pitch: 1.25,
            }
        );

        assert!(!audio.has_pending_events());
    }

    #[test]
    fn test_music_lifecycle_and_transitions() {
        let mut audio = AudioController::new();
        assert_eq!(audio.music_state(), MusicPlaybackState::Stopped);
        assert_eq!(audio.current_music(), None);

        // Inicia música do Santuário
        audio.play_music(MusicTrack::SanctuaryTheme, true);
        assert_eq!(audio.music_state(), MusicPlaybackState::Playing);
        assert_eq!(audio.current_music(), Some(MusicTrack::SanctuaryTheme));
        assert!(audio.is_music_looping());

        let events = audio.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            AudioEvent::PlayMusic {
                track: MusicTrack::SanctuaryTheme,
                loop_playback: true,
                volume: 0.8,
            }
        );

        // Tocar a mesma música não gera evento duplicado
        audio.play_music(MusicTrack::SanctuaryTheme, true);
        assert_eq!(audio.drain_events().len(), 0);

        // Pausa
        audio.pause_music();
        assert_eq!(audio.music_state(), MusicPlaybackState::Paused);
        let events = audio.drain_events();
        assert_eq!(events, vec![AudioEvent::PauseMusic]);

        // Retomada
        audio.resume_music();
        assert_eq!(audio.music_state(), MusicPlaybackState::Playing);
        let events = audio.drain_events();
        assert_eq!(events, vec![AudioEvent::ResumeMusic]);

        // Transição para Arena
        audio.play_music(MusicTrack::ArenaBattleTheme, true);
        assert_eq!(audio.current_music(), Some(MusicTrack::ArenaBattleTheme));
        let events = audio.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            AudioEvent::PlayMusic {
                track: MusicTrack::ArenaBattleTheme,
                loop_playback: true,
                volume: 0.8,
            }
        );

        // Parar
        audio.stop_music();
        assert_eq!(audio.music_state(), MusicPlaybackState::Stopped);
        assert_eq!(audio.current_music(), None);
        let events = audio.drain_events();
        assert_eq!(events, vec![AudioEvent::StopMusic]);
    }
}
