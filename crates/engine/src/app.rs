//! Abstração de aplicação e ciclo de vida multiplataforma.
#![forbid(unsafe_code)]

use crate::time::TimeStep;

/// Trait que define o ciclo de vida de um jogo em Bestigard.
pub trait GameState {
    /// Chamado uma vez durante a inicialização do jogo.
    fn init(&mut self) {}

    /// Atualização com passo fixo (física, produção de recursos, incubação, lógica determinística).
    fn fixed_update(&mut self, _fixed_dt: f32) {}

    /// Atualização com delta time variável (animações visuais, transições de câmera, interpolações).
    fn update(&mut self, _dt: f32) {}

    /// Renderização do estado atual, recebendo o fator alpha para interpolação entre ticks fixos.
    fn render(&mut self, _alpha: f32) {}

    /// Disparado quando o jogo perde foco ou é minimizado (ciclo de vida Android/iOS).
    fn on_pause(&mut self) {}

    /// Disparado quando o jogo volta ao primeiro plano.
    fn on_resume(&mut self) {}
}

/// Configuração do host da engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub target_ups: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Bestigard".to_string(),
            width: 1080,
            height: 1920,
            target_ups: 60.0,
        }
    }
}

/// Executor central da aplicação desacoplado de windowing nativo.
pub struct App<S: GameState> {
    config: EngineConfig,
    state: S,
    time: TimeStep,
    is_running: bool,
    is_paused: bool,
}

impl<S: GameState> App<S> {
    /// Inicializa uma nova aplicação com configuração e estado de jogo fornecidos.
    pub fn new(config: EngineConfig, mut state: S) -> Self {
        state.init();
        let target_ups = config.target_ups;

        Self {
            config,
            state,
            time: TimeStep::new(target_ups),
            is_running: true,
            is_paused: false,
        }
    }

    /// Executa um passo do ciclo de vida a partir do delta time em segundos decorrido do frame.
    /// Funciona perfeitamente em callbacks do Android NDK, iOS CADisplayLink ou loops desktop.
    pub fn step(&mut self, raw_dt: f32) {
        if !self.is_running || self.is_paused {
            return;
        }

        // 1. Atualizações em passos fixos (acumulador)
        let state = &mut self.state;
        self.time.advance(raw_dt, |fixed_dt| {
            state.fixed_update(fixed_dt);
        });

        // 2. Atualização variável de apresentação
        self.state.update(raw_dt);

        // 3. Renderização com alpha
        self.state.render(self.time.alpha());
    }

    /// Pausa a execução (por exemplo, ao enviar o app para background).
    pub fn pause(&mut self) {
        if !self.is_paused {
            self.is_paused = true;
            self.state.on_pause();
            self.time.reset_accumulator();
        }
    }

    /// Retoma a execução (ao reabrir o app).
    pub fn resume(&mut self) {
        if self.is_paused {
            self.is_paused = false;
            self.time.reset_accumulator();
            self.state.on_resume();
        }
    }

    /// Finaliza a execução do app.
    pub fn exit(&mut self) {
        self.is_running = false;
    }

    /// Retorna se a aplicação ainda está ativa.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Retorna se o app está em pausa.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    /// Acesso imutável à configuração da engine.
    #[inline]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Acesso imutável ao estado do jogo.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Acesso mutável ao estado do jogo.
    #[inline]
    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    /// Acesso imutável ao sistema de tempo.
    #[inline]
    pub fn time(&self) -> &TimeStep {
        &self.time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockGame {
        inited: bool,
        fixed_ticks: u32,
        updates: u32,
        renders: u32,
        pauses: u32,
    }

    impl GameState for MockGame {
        fn init(&mut self) {
            self.inited = true;
        }

        fn fixed_update(&mut self, _fixed_dt: f32) {
            self.fixed_ticks += 1;
        }

        fn update(&mut self, _dt: f32) {
            self.updates += 1;
        }

        fn render(&mut self, _alpha: f32) {
            self.renders += 1;
        }

        fn on_pause(&mut self) {
            self.pauses += 1;
        }
    }

    #[test]
    fn test_app_lifecycle() {
        let state = MockGame {
            inited: false,
            fixed_ticks: 0,
            updates: 0,
            renders: 0,
            pauses: 0,
        };

        let mut app = App::new(EngineConfig::default(), state);
        assert!(app.state().inited);

        // Executa 1 frame de 1/60 de segundo
        app.step(1.0 / 60.0);
        assert_eq!(app.state().fixed_ticks, 1);
        assert_eq!(app.state().updates, 1);
        assert_eq!(app.state().renders, 1);

        // Testa pausa
        app.pause();
        assert!(app.is_paused());
        assert_eq!(app.state().pauses, 1);

        // Enquanto pausado, step não avança lógica
        app.step(1.0);
        assert_eq!(app.state().fixed_ticks, 1);

        // Resume e avança novamente
        app.resume();
        assert!(!app.is_paused());
        app.step(1.0 / 60.0);
        assert_eq!(app.state().fixed_ticks, 2);
    }
}
