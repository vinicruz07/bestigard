//! Controle de tempo e game loop com passo fixo (Fixed Timestep Accumulator).
#![forbid(unsafe_code)]

/// Gerenciador de tempo para simulação física/econômica determinística com interpolação de render.
#[derive(Debug, Clone)]
pub struct TimeStep {
    /// Intervalo fixo para cada tick da lógica (ex: 1.0 / 60.0 para 60 ticks por segundo).
    pub fixed_dt: f32,
    /// Limite máximo aceito para um único delta frame, prevenindo a espiral da morte caso o jogo congele.
    pub max_dt: f32,
    /// Tempo acumulado ainda não processado em ticks fixos.
    accumulator: f32,
    /// Fator de interpolação (0.0 a 1.0) entre o tick anterior e o próximo, para suavização de render.
    alpha: f32,
    /// Quantidade total de ticks fixos executados desde o início.
    total_ticks: u64,
    /// Tempo total acumulado de simulação em segundos.
    total_time: f64,
}

impl TimeStep {
    /// Cria uma nova instância com a taxa de atualização física/econômica desejada.
    /// Exemplo: `TimeStep::new(60.0)` para 60Hz.
    pub fn new(target_ticks_per_second: f32) -> Self {
        let fixed_dt = if target_ticks_per_second <= 0.0 {
            1.0 / 60.0
        } else {
            1.0 / target_ticks_per_second
        };

        Self {
            fixed_dt,
            max_dt: 0.25, // Máximo de 250ms acumulados por frame (evita congelamento em pause)
            accumulator: 0.0,
            alpha: 0.0,
            total_ticks: 0,
            total_time: 0.0,
        }
    }

    /// Avança o relógio com o delta time do frame real e executa a closure `tick_fn`
    /// para cada passo fixo acumulado.
    /// Retorna a quantidade de ticks executados neste frame.
    pub fn advance<F>(&mut self, raw_dt: f32, mut tick_fn: F) -> usize
    where
        F: FnMut(f32),
    {
        // Limita o delta time para evitar espiral da morte
        let dt = raw_dt.max(0.0).min(self.max_dt);
        self.accumulator += dt;

        let mut ticks = 0;
        while self.accumulator >= self.fixed_dt {
            tick_fn(self.fixed_dt);
            self.accumulator -= self.fixed_dt;
            self.total_ticks += 1;
            self.total_time += self.fixed_dt as f64;
            ticks += 1;
        }

        // Alpha para interpolação de renderização
        self.alpha = if self.fixed_dt > 0.0 {
            self.accumulator / self.fixed_dt
        } else {
            0.0
        };

        ticks
    }

    /// Retorna o fator de interpolação entre o estado anterior e o estado atual (0.0 .. 1.0).
    #[inline]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Retorna o total de ticks executados.
    #[inline]
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }

    /// Retorna o tempo total de simulação acumulado em segundos.
    #[inline]
    pub fn total_time(&self) -> f64 {
        self.total_time
    }

    /// Reseta o acumulador (útil ao retomar o jogo após pause/suspensão no mobile).
    pub fn reset_accumulator(&mut self) {
        self.accumulator = 0.0;
        self.alpha = 0.0;
    }
}

impl Default for TimeStep {
    fn default() -> Self {
        Self::new(60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_timestep_tick_count() {
        let mut time = TimeStep::new(60.0);
        let mut count = 0;

        // Avança 60 frames regulares de 1/60s (totalizando 1 segundo)
        let frame_dt = 1.0 / 60.0;
        for _ in 0..60 {
            time.advance(frame_dt, |_dt| {
                count += 1;
            });
        }

        assert_eq!(count, 60);
        assert_eq!(time.total_ticks(), 60);
        assert!((time.total_time() - 1.0).abs() < 1e-4);
    }


    #[test]
    fn test_death_spiral_protection() {
        let mut time = TimeStep::new(60.0);
        let mut count = 0;

        // Simula pausa de 5 segundos no celular (dt = 5.0)
        time.advance(5.0, |_dt| {
            count += 1;
        });

        // max_dt é 0.25s -> no máximo 0.25 * 60 = 15 ticks
        assert_eq!(count, 15);
    }

    #[test]
    fn test_interpolation_alpha() {
        let mut time = TimeStep::new(10.0); // 0.1s por tick

        // Avança 0.05s (metade de um tick)
        let ticks = time.advance(0.05, |_| {});
        assert_eq!(ticks, 0);
        assert!((time.alpha() - 0.5).abs() < 1e-4);
    }
}
