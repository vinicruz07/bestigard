//! Sistema de Interface de Usuário (UI) leve, responsivo e modular em Safe Rust.
#![forbid(unsafe_code)]

use crate::font::BitmapFont;
use crate::geometry::{Point, Rect};
use crate::graphics::Color;
use crate::render::RenderQueue;
use crate::batch::SpriteBatch;

/// Paleta de cores moderna e temática para a interface de Bestigard.
pub struct UiColor;

impl UiColor {
    /// Fundo escuro translúcido com efeito de vidro fosco (glassmorphism).
    pub const DARK_GLASS: Color = Color::rgba(0.08, 0.12, 0.22, 0.94);
    /// Barra de cabeçalho de modais e divisores destacados.
    pub const HEADER_BG: Color = Color::rgba(0.12, 0.18, 0.32, 0.98);
    /// Borda dourada refinada para botões principais e contornos de painéis.
    pub const GOLD_ACCENT: Color = Color::rgba(0.95, 0.78, 0.25, 0.90);
    /// Fundo de botões comuns secundários.
    pub const BTN_SECONDARY: Color = Color::rgba(0.18, 0.28, 0.45, 0.95);
    /// Fundo de botões de ação positiva (Alimentar, Coletar, Cruzar).
    pub const BTN_PRIMARY: Color = Color::rgba(0.16, 0.65, 0.35, 0.95);
    /// Fundo de botões de destaque/inspeção (azul celeste).
    pub const BTN_ACCENT: Color = Color::rgba(0.20, 0.50, 0.85, 0.95);
    /// Fundo de botões de cancelamento ou fechar (vermelho carmim).
    pub const BTN_DANGER: Color = Color::rgba(0.75, 0.22, 0.22, 0.95);
    /// Botão desativado (opaco e acinzentado).
    pub const BTN_DISABLED: Color = Color::rgba(0.25, 0.25, 0.30, 0.60);
    /// Barra de preenchimento de XP / Nível.
    pub const XP_FILL: Color = Color::rgba(0.25, 0.70, 0.95, 0.95);
    /// Barra de preenchimento de Ouro / Recursos.
    pub const GOLD_FILL: Color = Color::rgba(1.0, 0.85, 0.20, 0.95);
    /// Escurecimento do cenário de fundo ao exibir janela modal (backdrop dimming).
    pub const BACKDROP_DIM: Color = Color::rgba(0.0, 0.0, 0.0, 0.65);
}

/// Ações originadas por interação do usuário com os elementos da interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    None,
    ButtonClicked(String),
    ModalClosed,
}

/// Botão interativo de interface com suporte a detecção de toque/clique e renderização.
#[derive(Debug, Clone)]
pub struct UiButton {
    pub id: String,
    pub rect: Rect,
    pub label: String,
    pub bg_color: Color,
    pub text_color: Color,
    pub text_scale: f32,
    pub is_disabled: bool,
}

impl UiButton {
    pub fn new(id: impl Into<String>, rect: Rect, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            rect,
            label: label.into(),
            bg_color: UiColor::BTN_PRIMARY,
            text_color: Color::WHITE,
            text_scale: 1.8,
            is_disabled: false,
        }
    }

    pub fn with_colors(mut self, bg_color: Color, text_color: Color) -> Self {
        self.bg_color = bg_color;
        self.text_color = text_color;
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.text_scale = scale;
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.is_disabled = disabled;
        self
    }

    /// Testa se o ponto (toque/cursor) está contido nos limites do botão.
    pub fn contains_point(&self, pt: Point) -> bool {
        self.rect.contains(pt.x, pt.y)
    }
}

/// Barra de progresso para níveis, tempo restante de incubação e colheitas.
#[derive(Debug, Clone)]
pub struct UiProgressBar {
    pub rect: Rect,
    pub ratio: f32,
    pub fill_color: Color,
    pub bg_color: Color,
    pub border_color: Option<Color>,
}

impl UiProgressBar {
    pub fn new(rect: Rect, ratio: f32, fill_color: Color) -> Self {
        Self {
            rect,
            ratio: ratio.clamp(0.0, 1.0),
            fill_color,
            bg_color: Color::rgba(0.05, 0.08, 0.15, 0.85),
            border_color: Some(Color::rgba(0.2, 0.3, 0.5, 0.8)),
        }
    }

    pub fn with_bg(mut self, bg_color: Color) -> Self {
        self.bg_color = bg_color;
        self
    }
}

/// Linha de texto formatada para exibição no interior de painéis e diálogos.
#[derive(Debug, Clone)]
pub struct UiTextLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub color: Color,
}

/// Janela modal interativa para inspeção de criaturas, seleção de cruzamento e habitats.
#[derive(Debug, Clone)]
pub struct UiModal {
    pub id: String,
    pub title: String,
    pub rect: Rect,
    pub close_button: UiButton,
    pub buttons: Vec<UiButton>,
    pub progress_bars: Vec<UiProgressBar>,
    pub text_lines: Vec<UiTextLine>,
    pub close_on_backdrop_tap: bool,
}

impl UiModal {
    pub fn new(id: impl Into<String>, title: impl Into<String>, rect: Rect) -> Self {
        let title_str = title.into();
        // Botão de fechar (X) posicionado no canto superior direito do modal
        let close_rect = Rect::new(rect.x + rect.w - 44.0, rect.y + 10.0, 34.0, 34.0);
        let close_btn = UiButton::new("btn_close", close_rect, "X")
            .with_colors(UiColor::BTN_DANGER, Color::WHITE)
            .with_scale(2.0);

        Self {
            id: id.into(),
            title: title_str,
            rect,
            close_button: close_btn,
            buttons: Vec::new(),
            progress_bars: Vec::new(),
            text_lines: Vec::new(),
            close_on_backdrop_tap: true,
        }
    }

    pub fn add_button(&mut self, button: UiButton) {
        self.buttons.push(button);
    }

    pub fn add_progress_bar(&mut self, bar: UiProgressBar) {
        self.progress_bars.push(bar);
    }

    pub fn add_text(
        &mut self,
        text: impl Into<String>,
        x: f32,
        y: f32,
        scale: f32,
        color: Color,
    ) {
        self.text_lines.push(UiTextLine {
            text: text.into(),
            x,
            y,
            scale,
            color,
        });
    }

    /// Processa o toque na janela modal.
    pub fn handle_tap(&self, pt: Point) -> UiAction {
        // 1. Verifica botão de fechar
        if self.close_button.contains_point(pt) {
            return UiAction::ModalClosed;
        }

        // 2. Verifica botões de ação do diálogo
        for btn in &self.buttons {
            if !btn.is_disabled && btn.contains_point(pt) {
                return UiAction::ButtonClicked(btn.id.clone());
            }
        }

        // 3. Toque fora do modal fecha se permitido
        if self.close_on_backdrop_tap && !self.rect.contains(pt.x, pt.y) {
            return UiAction::ModalClosed;
        }

        UiAction::None
    }
}

/// Gerenciador central de interface de usuário (HUD, Modais e Botões Globais).
#[derive(Debug, Clone)]
pub struct UiManager {
    pub screen_width: f32,
    pub screen_height: f32,
    pub global_buttons: Vec<UiButton>,
    pub active_modal: Option<UiModal>,
}

impl UiManager {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            global_buttons: Vec::new(),
            active_modal: None,
        }
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    pub fn add_global_button(&mut self, btn: UiButton) {
        self.global_buttons.push(btn);
    }

    pub fn clear_global_buttons(&mut self) {
        self.global_buttons.clear();
    }

    pub fn open_modal(&mut self, modal: UiModal) {
        self.active_modal = Some(modal);
    }

    pub fn close_modal(&mut self) {
        self.active_modal = None;
    }

    pub fn is_modal_open(&self) -> bool {
        self.active_modal.is_some()
    }

    /// Despacha toque de usuário respeitando a modalidade da interface.
    /// Se um modal estiver aberto, nenhum botão de fundo é acionado.
    pub fn handle_tap(&mut self, pt: Point) -> UiAction {
        if let Some(ref modal) = self.active_modal {
            let action = modal.handle_tap(pt);
            if action == UiAction::ModalClosed {
                self.active_modal = None;
            }
            return action;
        }

        // Sem modal ativo: processa botões globais do HUD (ordem inversa: mais recentes primeiro)
        for btn in self.global_buttons.iter().rev() {
            if !btn.is_disabled && btn.contains_point(pt) {
                return UiAction::ButtonClicked(btn.id.clone());
            }
        }

        UiAction::None
    }

    /// Enfileira comandos de renderização na fila de camadas e renderiza texto com BitmapFont.
    pub fn render(
        &self,
        render_queue: &mut RenderQueue,
        batch: &mut SpriteBatch,
        font: &BitmapFont,
    ) {
        // 1. Renderiza botões globais do HUD
        for btn in &self.global_buttons {
            self.draw_button(btn, render_queue, batch, font);
        }

        // 2. Renderiza modal ativo com backdrop escurecido
        if let Some(ref modal) = self.active_modal {
            // Escurecimento do cenário de fundo
            render_queue.push_ui_rect(
                Rect::new(0.0, 0.0, self.screen_width, self.screen_height),
                UiColor::BACKDROP_DIM,
            );

            // Borda externa decorativa dourada
            render_queue.push_ui_rect(
                Rect::new(
                    modal.rect.x - 3.0,
                    modal.rect.y - 3.0,
                    modal.rect.w + 6.0,
                    modal.rect.h + 6.0,
                ),
                UiColor::GOLD_ACCENT,
            );

            // Fundo principal escuro com efeito de vidro
            render_queue.push_ui_rect(modal.rect, UiColor::DARK_GLASS);

            // Barra de cabeçalho
            let header_rect = Rect::new(modal.rect.x, modal.rect.y, modal.rect.w, 54.0);
            render_queue.push_ui_rect(header_rect, UiColor::HEADER_BG);

            // Barras de progresso do modal
            for bar in &modal.progress_bars {
                self.draw_progress_bar(bar, render_queue);
            }

            // Botão de fechar (X)
            self.draw_button(&modal.close_button, render_queue, batch, font);

            // Botões de ação do modal
            for btn in &modal.buttons {
                self.draw_button(btn, render_queue, batch, font);
            }
        }

        // Efetua flush dos comandos da UI para o batch antes de desenhar textos
        render_queue.flush_to_batch(batch);

        // 3. Renderiza textos tipográficos por cima dos painéis (BitmapFont)
        // Textos dos botões globais
        for btn in &self.global_buttons {
            self.draw_button_text(btn, batch, font);
        }

        if let Some(ref modal) = self.active_modal {
            // Título do modal no cabeçalho
            font.draw_text(
                batch,
                &modal.title,
                modal.rect.x + 20.0,
                modal.rect.y + 18.0,
                2.2,
                UiColor::GOLD_ACCENT,
            );

            // Texto do botão de fechar
            self.draw_button_text(&modal.close_button, batch, font);

            // Linhas de texto internas do diálogo
            for line in &modal.text_lines {
                font.draw_text(batch, &line.text, line.x, line.y, line.scale, line.color);
            }

            // Textos dos botões do modal
            for btn in &modal.buttons {
                self.draw_button_text(btn, batch, font);
            }
        }
    }

    fn draw_button(
        &self,
        btn: &UiButton,
        render_queue: &mut RenderQueue,
        _batch: &mut SpriteBatch,
        _font: &BitmapFont,
    ) {
        let color = if btn.is_disabled {
            UiColor::BTN_DISABLED
        } else {
            btn.bg_color
        };

        // Borda fina decorativa
        render_queue.push_ui_rect(
            Rect::new(
                btn.rect.x - 2.0,
                btn.rect.y - 2.0,
                btn.rect.w + 4.0,
                btn.rect.h + 4.0,
            ),
            Color::rgba(0.0, 0.0, 0.0, 0.5),
        );

        // Corpo do botão
        render_queue.push_ui_rect(btn.rect, color);
    }

    fn draw_button_text(&self, btn: &UiButton, batch: &mut SpriteBatch, font: &BitmapFont) {
        let (tw, th) = font.measure_text(&btn.label, btn.text_scale);
        let tx = btn.rect.x + (btn.rect.w - tw) * 0.5;
        let ty = btn.rect.y + (btn.rect.h - th) * 0.5;

        let text_color = if btn.is_disabled {
            Color::rgba(0.7, 0.7, 0.7, 0.6)
        } else {
            btn.text_color
        };

        font.draw_text(batch, &btn.label, tx, ty, btn.text_scale, text_color);
    }

    fn draw_progress_bar(&self, bar: &UiProgressBar, render_queue: &mut RenderQueue) {
        // Fundo da barra
        render_queue.push_ui_rect(bar.rect, bar.bg_color);

        // Preenchimento com base no ratio
        if bar.ratio > 0.0 {
            let fill_width = (bar.rect.w * bar.ratio).max(2.0);
            let fill_rect = Rect::new(bar.rect.x, bar.rect.y, fill_width, bar.rect.h);
            render_queue.push_ui_rect(fill_rect, bar.fill_color);
        }

        // Borda opcional
        if let Some(border) = bar.border_color {
            // Contorno superior e inferior
            render_queue.push_ui_rect(
                Rect::new(bar.rect.x, bar.rect.y, bar.rect.w, 2.0),
                border,
            );
            render_queue.push_ui_rect(
                Rect::new(bar.rect.x, bar.rect.y + bar.rect.h - 2.0, bar.rect.w, 2.0),
                border,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_button_hit_test() {
        let btn = UiButton::new("btn_play", Rect::new(100.0, 100.0, 200.0, 50.0), "JOGAR");
        assert!(btn.contains_point(Point::new(150.0, 120.0)));
        assert!(btn.contains_point(Point::new(100.0, 100.0)));
        assert!(!btn.contains_point(Point::new(99.0, 100.0)));
        assert!(!btn.contains_point(Point::new(350.0, 120.0)));
    }

    #[test]
    fn test_ui_modal_blocking_and_actions() {
        let mut ui = UiManager::new(1080.0, 1920.0);

        // Adiciona botão no HUD de fundo
        let hud_btn = UiButton::new("btn_hud_collect", Rect::new(20.0, 20.0, 120.0, 40.0), "COLETAR");
        ui.add_global_button(hud_btn);

        // Sem modal: clique no botão do HUD deve funcionar
        let act1 = ui.handle_tap(Point::new(50.0, 30.0));
        assert_eq!(act1, UiAction::ButtonClicked("btn_hud_collect".to_string()));

        // Abre modal de criatura
        let mut modal = UiModal::new(
            "modal_creature",
            "INSPECIONAR SALAMANDRA",
            Rect::new(140.0, 400.0, 800.0, 600.0),
        );
        let feed_btn = UiButton::new("btn_feed", Rect::new(200.0, 850.0, 300.0, 60.0), "ALIMENTAR");
        modal.add_button(feed_btn);
        ui.open_modal(modal);

        assert!(ui.is_modal_open());

        // Com modal aberto: toque no botão do HUD NÃO deve funcionar (bloqueio modal)
        let act2 = ui.handle_tap(Point::new(50.0, 30.0));
        // Como o backdrop tap fecha o modal quando clica fora, deve retornar ModalClosed
        assert_eq!(act2, UiAction::ModalClosed);
        assert!(!ui.is_modal_open());

        // Reabre modal e clica no botão interno "ALIMENTAR"
        let mut modal2 = UiModal::new(
            "modal_creature",
            "INSPECIONAR SALAMANDRA",
            Rect::new(140.0, 400.0, 800.0, 600.0),
        );
        modal2.add_button(UiButton::new("btn_feed", Rect::new(200.0, 850.0, 300.0, 60.0), "ALIMENTAR"));
        ui.open_modal(modal2);

        let act3 = ui.handle_tap(Point::new(250.0, 870.0));
        assert_eq!(act3, UiAction::ButtonClicked("btn_feed".to_string()));
        assert!(ui.is_modal_open()); // Modal continua aberto após alimentar
    }

    #[test]
    fn test_ui_modal_close_button() {
        let mut ui = UiManager::new(1080.0, 1920.0);
        let modal = UiModal::new("modal_test", "TESTE", Rect::new(100.0, 100.0, 400.0, 300.0));
        let close_pt = Point::new(modal.close_button.rect.x + 5.0, modal.close_button.rect.y + 5.0);
        ui.open_modal(modal);

        let act = ui.handle_tap(close_pt);
        assert_eq!(act, UiAction::ModalClosed);
        assert!(!ui.is_modal_open());
    }

    #[test]
    fn test_ui_progress_bar_clamping() {
        let bar = UiProgressBar::new(Rect::new(0.0, 0.0, 100.0, 20.0), 1.5, Color::WHITE);
        assert_eq!(bar.ratio, 1.0);

        let bar_neg = UiProgressBar::new(Rect::new(0.0, 0.0, 100.0, 20.0), -0.5, Color::WHITE);
        assert_eq!(bar_neg.ratio, 0.0);
    }
}
