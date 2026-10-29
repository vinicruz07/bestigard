//! Motor de combate em turnos e matriz de eficácia elemental.
#![forbid(unsafe_code)]

use crate::creature::{CreatureInstance, CreatureSpecies};
use crate::element::ElementId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Multiplicador de eficácia de um elemento atacante contra um elemento defensor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Effectiveness {
    Immune,       // 0.0x
    Resisted,     // 0.5x
    Neutral,      // 1.0x
    SuperEffective, // 2.0x
}

impl Effectiveness {
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Immune => 0.0,
            Self::Resisted => 0.5,
            Self::Neutral => 1.0,
            Self::SuperEffective => 2.0,
        }
    }

    /// Rótulo descritivo de vantagem tática em combate.
    pub fn tactical_label(&self) -> &'static str {
        match self {
            Self::Immune => "0.0x IMUNE",
            Self::Resisted => "0.5x RESISTIDO",
            Self::Neutral => "1.0x NEUTRO",
            Self::SuperEffective => "2.0x SUPER EFETIVO!",
        }
    }
}

/// Matriz de afinidades e fraquezas elementais orientada a dados.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementalChart {
    affinities: HashMap<(ElementId, ElementId), Effectiveness>,
}

impl ElementalChart {
    pub fn new() -> Self {
        Self {
            affinities: HashMap::new(),
        }
    }

    /// Cria a tabela de afinidades padrão equilibrada para os 7 elementos fundamentais.
    pub fn with_default_affinities() -> Self {
        let mut chart = Self::new();

        let fire = ElementId::new("fire");
        let water = ElementId::new("water");
        let air = ElementId::new("air");
        let earth = ElementId::new("earth");
        let lightning = ElementId::new("lightning");
        let light = ElementId::new("light");
        let shadow = ElementId::new("shadow");

        // FOGO
        chart.set_affinity(&fire, &earth, Effectiveness::SuperEffective);
        chart.set_affinity(&fire, &air, Effectiveness::SuperEffective);
        chart.set_affinity(&fire, &water, Effectiveness::Resisted);

        // ÁGUA
        chart.set_affinity(&water, &fire, Effectiveness::SuperEffective);
        chart.set_affinity(&water, &earth, Effectiveness::SuperEffective);
        chart.set_affinity(&water, &lightning, Effectiveness::Resisted);

        // AR
        chart.set_affinity(&air, &water, Effectiveness::SuperEffective);
        chart.set_affinity(&air, &lightning, Effectiveness::Resisted);

        // TERRA
        chart.set_affinity(&earth, &lightning, Effectiveness::Immune);
        chart.set_affinity(&earth, &fire, Effectiveness::SuperEffective);
        chart.set_affinity(&earth, &water, Effectiveness::Resisted);

        // RAIO
        chart.set_affinity(&lightning, &water, Effectiveness::SuperEffective);
        chart.set_affinity(&lightning, &air, Effectiveness::SuperEffective);
        chart.set_affinity(&lightning, &earth, Effectiveness::Immune);

        // LUZ E SOMBRA (Dano mútuo sagrado vs abissal)
        chart.set_affinity(&light, &shadow, Effectiveness::SuperEffective);
        chart.set_affinity(&shadow, &light, Effectiveness::SuperEffective);
        chart.set_affinity(&light, &light, Effectiveness::Resisted);
        chart.set_affinity(&shadow, &shadow, Effectiveness::Resisted);

        chart
    }

    /// Registra ou substitui a relação entre um elemento atacante e um defensor.
    pub fn set_affinity(
        &mut self,
        attacker: &ElementId,
        defender: &ElementId,
        effectiveness: Effectiveness,
    ) {
        self.affinities.insert((attacker.clone(), defender.clone()), effectiveness);
    }

    /// Retorna a eficácia do ataque contra um elemento do defensor (Neutral por padrão se não definido).
    pub fn get_effectiveness(&self, attacker: &ElementId, defender: &ElementId) -> Effectiveness {
        self.affinities
            .get(&(attacker.clone(), defender.clone()))
            .copied()
            .unwrap_or(Effectiveness::Neutral)
    }

    /// Calcula o multiplicador combinado contra uma criatura com múltiplos elementos.
    pub fn calculate_multiplier(&self, attacker_element: &ElementId, defender_elements: &[ElementId]) -> f32 {
        if defender_elements.is_empty() {
            return 1.0;
        }

        let mut multiplier = 1.0f32;
        for def_el in defender_elements {
            multiplier *= self.get_effectiveness(attacker_element, def_el).multiplier();
        }

        multiplier
    }

    /// Retorna o rótulo descritivo formatado do multiplicador tático combinado.
    pub fn tactical_label(&self, attacker_element: &ElementId, defender_elements: &[ElementId]) -> &'static str {
        let mult = self.calculate_multiplier(attacker_element, defender_elements);
        if mult >= 2.0 {
            "2.0x SUPER EFETIVO!"
        } else if mult == 0.0 {
            "0.0x IMUNE"
        } else if mult < 1.0 {
            "0.5x RESISTIDO"
        } else {
            "1.0x NEUTRO"
        }
    }
}

/// Habilidade elemental de combate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatSkill {
    pub id: String,
    pub name: String,
    pub element: ElementId,
    pub base_power: u32,
    pub cooldown_turns: u32,
}

impl CombatSkill {
    pub fn basic_strike(element: &ElementId) -> Self {
        Self {
            id: format!("{}_strike", element.as_str()),
            name: format!("Golpe de {}", element.as_str()),
            element: element.clone(),
            base_power: 40,
            cooldown_turns: 0,
        }
    }

    pub fn heavy_burst(element: &ElementId) -> Self {
        Self {
            id: format!("{}_burst", element.as_str()),
            name: format!("Explosão de {}", element.as_str()),
            element: element.clone(),
            base_power: 80,
            cooldown_turns: 2,
        }
    }
}

/// Estado de um lutador dentro de uma batalha em turnos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Combatant {
    pub combatant_id: u64,
    pub species_id: String,
    pub name: String,
    pub elements: Vec<ElementId>,
    pub current_hp: u32,
    pub max_hp: u32,
    pub attack: u32,
    pub defense: u32,
    pub speed: u32,
    pub skills: Vec<CombatSkill>,
    pub skill_cooldowns: HashMap<String, u32>,
}

impl Combatant {
    /// Inicializa o combatente a partir da criatura e espécie do jogador.
    pub fn from_creature(
        creature: &CreatureInstance,
        species: &CreatureSpecies,
    ) -> Self {
        let max_hp = creature.calculate_hp(species);
        let attack = creature.calculate_attack(species);
        let defense = creature.calculate_defense(species);
        let speed = creature.calculate_speed(species);

        // Habilidades base baseadas nos elementos da criatura
        let mut skills = Vec::new();
        for el in &species.elements {
            skills.push(CombatSkill::basic_strike(el));
            skills.push(CombatSkill::heavy_burst(el));
        }

        Self {
            combatant_id: creature.instance_id,
            species_id: species.id.clone(),
            name: creature.nickname.clone().unwrap_or_else(|| species.name.clone()),
            elements: species.elements.clone(),
            current_hp: max_hp,
            max_hp,
            attack,
            defense,
            speed,
            skills,
            skill_cooldowns: HashMap::new(),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    /// Reduz HP sofrendo dano e retorna o dano real absorvido.
    pub fn take_damage(&mut self, amount: u32) -> u32 {
        let damage = amount.min(self.current_hp);
        self.current_hp -= damage;
        damage
    }

    /// Reduz cooldowns no início de um novo turno.
    pub fn tick_cooldowns(&mut self) {
        for cd in self.skill_cooldowns.values_mut() {
            if *cd > 0 {
                *cd -= 1;
            }
        }
    }
}

/// Registro de um evento ocorrido durante a batalha.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatActionLog {
    pub attacker_name: String,
    pub target_name: String,
    pub skill_name: String,
    pub elemental_multiplier: f32,
    pub raw_damage: u32,
    pub target_remaining_hp: u32,
    pub target_defeated: bool,
}

/// Resultado final de uma sessão de combate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleOutcome {
    Ongoing,
    PlayerVictory,
    EnemyVictory,
    Draw,
}

/// Sessão completa de combate em turnos (equipe do jogador vs equipe adversária).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleSession {
    pub player_team: Vec<Combatant>,
    pub enemy_team: Vec<Combatant>,
    pub current_turn: u32,
    pub is_finished: bool,
    pub outcome: BattleOutcome,
}

impl BattleSession {
    pub fn new(player_team: Vec<Combatant>, enemy_team: Vec<Combatant>) -> Self {
        Self {
            player_team,
            enemy_team,
            current_turn: 1,
            is_finished: false,
            outcome: BattleOutcome::Ongoing,
        }
    }

    /// Retorna o lutador ativo do jogador (o primeiro ainda vivo).
    pub fn active_player_combatant(&self) -> Option<&Combatant> {
        self.player_team.iter().find(|c| c.is_alive())
    }

    /// Retorna o lutador ativo do adversário (o primeiro ainda vivo).
    pub fn active_enemy_combatant(&self) -> Option<&Combatant> {
        self.enemy_team.iter().find(|c| c.is_alive())
    }

    /// Executa um ataque de um combatente contra o alvo, calculando eficácia elemental e dano.
    pub fn execute_attack(
        attacker: &mut Combatant,
        target: &mut Combatant,
        skill_index: usize,
        chart: &ElementalChart,
    ) -> CombatActionLog {
        let skill = attacker.skills.get(skill_index).cloned().unwrap_or_else(|| {
            CombatSkill::basic_strike(&attacker.elements[0])
        });

        // 1. Multiplicador elemental
        let elem_mult = chart.calculate_multiplier(&skill.element, &target.elements);

        // 2. Cálculo da fórmula clássica de RPG balanceada: (Ataque * Poder / Defesa) * Multiplicador
        let base_calc = ((attacker.attack as f32 * skill.base_power as f32) / (target.defense.max(1) as f32)).round();
        let final_damage = (base_calc * elem_mult).max(1.0).round() as u32;

        target.take_damage(final_damage);

        // 3. Aplica cooldown na habilidade se houver
        if skill.cooldown_turns > 0 {
            attacker.skill_cooldowns.insert(skill.id.clone(), skill.cooldown_turns);
        }

        CombatActionLog {
            attacker_name: attacker.name.clone(),
            target_name: target.name.clone(),
            skill_name: skill.name,
            elemental_multiplier: elem_mult,
            raw_damage: final_damage,
            target_remaining_hp: target.current_hp,
            target_defeated: !target.is_alive(),
        }
    }

    /// Executa uma rodada completa de combate em turnos entre os combatentes ativos.
    pub fn execute_round(
        &mut self,
        player_skill_idx: usize,
        chart: &ElementalChart,
    ) -> (Option<CombatActionLog>, Option<CombatActionLog>) {
        if self.is_finished {
            return (None, None);
        }

        // Determina quem ataca primeiro com base na Velocidade (Speed)
        let player_faster = {
            let p_speed = self.active_player_combatant().map(|c| c.speed).unwrap_or(0);
            let e_speed = self.active_enemy_combatant().map(|c| c.speed).unwrap_or(0);
            p_speed >= e_speed
        };

        let mut first_log = None;
        let mut second_log = None;

        if player_faster {
            // Jogador ataca primeiro
            if let (Some(p_idx), Some(e_idx)) = (self.first_alive_player_idx(), self.first_alive_enemy_idx()) {
                let log = Self::execute_attack(
                    &mut self.player_team[p_idx],
                    &mut self.enemy_team[e_idx],
                    player_skill_idx,
                    chart,
                );
                first_log = Some(log);

                // Se o inimigo sobreviveu, contra-ataca
                if self.enemy_team[e_idx].is_alive() {
                    let enemy_log = Self::execute_attack(
                        &mut self.enemy_team[e_idx],
                        &mut self.player_team[p_idx],
                        0, // IA usa ataque básico
                        chart,
                    );
                    second_log = Some(enemy_log);
                }
            }
        } else {
            // Inimigo ataca primeiro
            if let (Some(p_idx), Some(e_idx)) = (self.first_alive_player_idx(), self.first_alive_enemy_idx()) {
                let log = Self::execute_attack(
                    &mut self.enemy_team[e_idx],
                    &mut self.player_team[p_idx],
                    0,
                    chart,
                );
                first_log = Some(log);

                // Se o jogador sobreviveu, ataca
                if self.player_team[p_idx].is_alive() {
                    let player_log = Self::execute_attack(
                        &mut self.player_team[p_idx],
                        &mut self.enemy_team[e_idx],
                        player_skill_idx,
                        chart,
                    );
                    second_log = Some(player_log);
                }
            }
        }

        // Atualiza cooldowns e verifica desfecho da partida
        self.update_battle_state();
        self.current_turn += 1;

        (first_log, second_log)
    }

    fn first_alive_player_idx(&self) -> Option<usize> {
        self.player_team.iter().position(|c| c.is_alive())
    }

    fn first_alive_enemy_idx(&self) -> Option<usize> {
        self.enemy_team.iter().position(|c| c.is_alive())
    }

    fn update_battle_state(&mut self) {
        let player_alive = self.player_team.iter().any(|c| c.is_alive());
        let enemy_alive = self.enemy_team.iter().any(|c| c.is_alive());

        if !player_alive && !enemy_alive {
            self.is_finished = true;
            self.outcome = BattleOutcome::Draw;
        } else if !enemy_alive {
            self.is_finished = true;
            self.outcome = BattleOutcome::PlayerVictory;
        } else if !player_alive {
            self.is_finished = true;
            self.outcome = BattleOutcome::EnemyVictory;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::SpeciesCatalog;

    #[test]
    fn test_elemental_chart_multipliers() {
        let chart = ElementalChart::with_default_affinities();

        let fire = ElementId::new("fire");
        let water = ElementId::new("water");
        let earth = ElementId::new("earth");
        let lightning = ElementId::new("lightning");

        // Fogo em Água deve ser resistido (0.5x)
        assert_eq!(chart.calculate_multiplier(&fire, &[water.clone()]), 0.5);

        // Água em Fogo deve ser super efetivo (2.0x)
        assert_eq!(chart.calculate_multiplier(&water, &[fire.clone()]), 2.0);

        // Raio em Terra deve ser imune (0.0x)
        assert_eq!(chart.calculate_multiplier(&lightning, &[earth.clone()]), 0.0);

        // Água em criatura híbrida (Fogo + Terra) = 2.0 * 2.0 = 4.0x de dano quadruplicado!
        assert_eq!(chart.calculate_multiplier(&water, &[fire.clone(), earth.clone()]), 4.0);

        // Feedback de vantagem tática
        assert_eq!(chart.tactical_label(&water, &[fire.clone()]), "2.0x SUPER EFETIVO!");
        assert_eq!(chart.tactical_label(&fire, &[water.clone()]), "0.5x RESISTIDO");
        assert_eq!(chart.tactical_label(&lightning, &[earth.clone()]), "0.0x IMUNE");
        assert_eq!(Effectiveness::SuperEffective.tactical_label(), "2.0x SUPER EFETIVO!");
    }

    #[test]
    fn test_turn_based_battle_execution() {
        let catalog = SpeciesCatalog::with_starter_species();
        let chart = ElementalChart::with_default_affinities();

        let pyro = catalog.get("pyro_salamander").unwrap();
        let water = catalog.get("tide_serpent").unwrap();

        let mut p_inst = CreatureInstance::new(1, &pyro.id);
        p_inst.add_xp(1000); // Nível 4+

        let e_inst = CreatureInstance::new(2, &water.id);

        let p_combatant = Combatant::from_creature(&p_inst, pyro);
        let e_combatant = Combatant::from_creature(&e_inst, water);

        let mut session = BattleSession::new(vec![p_combatant], vec![e_combatant]);
        assert_eq!(session.outcome, BattleOutcome::Ongoing);

        // Executa rodadas até que um lado seja derrotado
        let mut rounds = 0;
        while !session.is_finished && rounds < 20 {
            session.execute_round(1, &chart); // Usa habilidade 1
            rounds += 1;
        }

        assert!(session.is_finished);
        assert!(session.outcome == BattleOutcome::PlayerVictory || session.outcome == BattleOutcome::EnemyVictory);
    }
}
