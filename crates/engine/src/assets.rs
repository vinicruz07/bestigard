//! Carregador multiplataforma de assets (Desktop Filesystem + Android APK + Embedded Fallback Atlas).
#![forbid(unsafe_code)]

use crate::atlas::TextureAtlas;
use crate::font::BitmapFont;
use crate::geometry::Rect;
use crate::texture::RgbaImage;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// Origem de onde um asset foi efetivamente carregado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetSource {
    /// Carregado diretamente do sistema de arquivos (desenvolvimento desktop).
    Filesystem(PathBuf),
    /// Carregado do pacote de assets de um APK Android.
    AndroidApk(String),
    /// Carregado da memória embutida no binário ou gerador procedural de fallback.
    Embedded(String),
}

/// Erros de leitura ou decodificação de assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    NotFound(String),
    Io(String),
    Decode(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "Asset não encontrado: '{}'", p),
            Self::Io(e) => write!(f, "Erro de I/O ao ler asset: {}", e),
            Self::Decode(e) => write!(f, "Erro ao decodificar asset: {}", e),
        }
    }
}

impl std::error::Error for AssetError {}

/// Tipo de recurso solicitado em uma requisição assíncrona/incremental.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetRequestKind {
    TexturePng { fallback_w: u32, fallback_h: u32 },
    AtlasManifest,
    RawBytes,
    TextFile,
}

/// Estado do ciclo de vida de carregamento de um asset na fila.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetLoadState {
    Queued,
    Ready(AssetSource),
    FallbackUsed(AssetSource),
    Failed(String),
}

/// Requisição individual na fila de carregamento incremental.
#[derive(Debug, Clone)]
pub struct AssetLoadRequest {
    pub id: String,
    pub relative_path: String,
    pub kind: AssetRequestKind,
}

/// Pacote consolidado do Atlas do Santuário pronto para upload na GPU.
#[derive(Debug, Clone)]
pub struct SanctuaryAtlasBundle {
    pub atlas: TextureAtlas,
    pub image: RgbaImage,
    pub source: AssetSource,
    pub external_sprites_loaded: usize,
}

/// Gerenciador e carregador multiplataforma de assets com suporte a fila incremental e fallback embutido.
#[derive(Debug, Clone)]
pub struct AssetLoader {
    assets_root: PathBuf,
    embedded_blobs: HashMap<String, Vec<u8>>,
    load_queue: VecDeque<AssetLoadRequest>,
    states: HashMap<String, AssetLoadState>,
    loaded_textures: HashMap<String, RgbaImage>,
    loaded_atlases: HashMap<String, TextureAtlas>,
    loaded_texts: HashMap<String, String>,
    loaded_bytes: HashMap<String, Vec<u8>>,
    total_queued: usize,
    total_completed: usize,
}

impl AssetLoader {
    pub const DEFAULT_ASSETS_DIR: &'static str = "assets";
    pub const EMBEDDED_ATLAS_WIDTH: u32 = 512;
    pub const EMBEDDED_ATLAS_HEIGHT: u32 = 512;

    /// Cria um carregador configurado para a plataforma atual.
    pub fn for_current_platform() -> Self {
        Self::new(Self::DEFAULT_ASSETS_DIR)
    }

    /// Cria um carregador apontando para um diretório raiz de assets específico.
    pub fn new(assets_root: impl Into<PathBuf>) -> Self {
        Self {
            assets_root: assets_root.into(),
            embedded_blobs: HashMap::new(),
            load_queue: VecDeque::new(),
            states: HashMap::new(),
            loaded_textures: HashMap::new(),
            loaded_atlases: HashMap::new(),
            loaded_texts: HashMap::new(),
            loaded_bytes: HashMap::new(),
            total_queued: 0,
            total_completed: 0,
        }
    }

    #[inline]
    pub fn assets_root(&self) -> &Path {
        &self.assets_root
    }

    /// Registra um blob binário em memória (útil para assets empacotados no executável/APK).
    pub fn register_embedded_bytes(&mut self, relative_path: impl Into<String>, bytes: Vec<u8>) {
        let normalized = Self::normalize_path(&relative_path.into());
        self.embedded_blobs.insert(normalized, bytes);
    }

    /// Normaliza separadores de caminho para formato portátil (`/`).
    pub fn normalize_path(path: &str) -> String {
        path.trim_start_matches("./")
            .trim_start_matches(".\\")
            .replace('\\', "/")
    }

    /// Carrega bytes brutos de um asset priorizando:
    /// 1. Blobs embutidos registrados em memória (APK / embedded);
    /// 2. Caminhos do sistema de arquivos (`assets_root/relative_path` ou `relative_path`);
    /// 3. Caminhos internos do pacote Android quando compilado para target Android.
    pub fn load_bytes(&self, relative_path: &str) -> Result<(Vec<u8>, AssetSource), AssetError> {
        let norm = Self::normalize_path(relative_path);

        if let Some(blob) = self.embedded_blobs.get(&norm) {
            return Ok((blob.clone(), AssetSource::Embedded(norm)));
        }

        let primary_path = self.assets_root.join(&norm);
        if primary_path.exists() {
            let bytes = fs::read(&primary_path).map_err(|e| AssetError::Io(e.to_string()))?;
            return Ok((bytes, AssetSource::Filesystem(primary_path)));
        }

        let direct_path = PathBuf::from(&norm);
        if direct_path.exists() {
            let bytes = fs::read(&direct_path).map_err(|e| AssetError::Io(e.to_string()))?;
            return Ok((bytes, AssetSource::Filesystem(direct_path)));
        }

        #[cfg(target_os = "android")]
        {
            let apk_candidate = PathBuf::from("/data/data/com.bestigard.game/files/assets").join(&norm);
            if apk_candidate.exists() {
                let bytes = fs::read(&apk_candidate).map_err(|e| AssetError::Io(e.to_string()))?;
                return Ok((bytes, AssetSource::AndroidApk(norm)));
            }
        }

        Err(AssetError::NotFound(norm))
    }

    /// Carrega um arquivo de texto UTF-8 (ex: JSON de manifesto, configuração ou localização).
    pub fn load_text(&self, relative_path: &str) -> Result<(String, AssetSource), AssetError> {
        let (bytes, source) = self.load_bytes(relative_path)?;
        let text = String::from_utf8(bytes).map_err(|e| AssetError::Decode(e.to_string()))?;
        Ok((text, source))
    }

    /// Carrega uma textura PNG ou gera automaticamente uma textura procedural caso o arquivo não esteja presente.
    pub fn load_png_or_fallback(
        &self,
        relative_path: &str,
        fallback_w: u32,
        fallback_h: u32,
    ) -> (RgbaImage, AssetSource, bool) {
        if let Ok((bytes, source)) = self.load_bytes(relative_path) {
            if let Ok(img) = RgbaImage::from_png_bytes(&bytes) {
                return (img, source, false);
            }
        }

        let fallback = RgbaImage::checkerboard(
            fallback_w.max(1),
            fallback_h.max(1),
            8,
            [220, 80, 220, 255],
            [40, 40, 55, 255],
        );
        (
            fallback,
            AssetSource::Embedded(format!("fallback:{}", Self::normalize_path(relative_path))),
            true,
        )
    }

    /// Carrega um `TextureAtlas` serializado em JSON.
    pub fn load_atlas_manifest(
        &self,
        relative_path: &str,
    ) -> Result<(TextureAtlas, AssetSource), AssetError> {
        let (json_text, source) = self.load_text(relative_path)?;
        let atlas = TextureAtlas::from_json(&json_text).map_err(AssetError::Decode)?;
        Ok((atlas, source))
    }

    /// Enfileira uma requisição de carregamento assíncrono/incremental.
    pub fn enqueue(&mut self, id: impl Into<String>, relative_path: impl Into<String>, kind: AssetRequestKind) {
        let id_str = id.into();
        let req = AssetLoadRequest {
            id: id_str.clone(),
            relative_path: relative_path.into(),
            kind,
        };
        self.states.insert(id_str, AssetLoadState::Queued);
        self.load_queue.push_back(req);
        self.total_queued += 1;
    }

    /// Processa até `max_items` da fila de carregamento neste frame (evita travamentos de frame no mobile).
    /// Retorna quantos itens foram processados nesta chamada.
    pub fn poll_load_queue(&mut self, max_items: usize) -> usize {
        let limit = max_items.max(1);
        let mut processed = 0;

        while processed < limit {
            let Some(req) = self.load_queue.pop_front() else {
                break;
            };
            processed += 1;
            self.total_completed += 1;

            match req.kind {
                AssetRequestKind::TexturePng {
                    fallback_w,
                    fallback_h,
                } => {
                    let (img, source, used_fallback) =
                        self.load_png_or_fallback(&req.relative_path, fallback_w, fallback_h);
                    self.loaded_textures.insert(req.id.clone(), img);
                    let state = if used_fallback {
                        AssetLoadState::FallbackUsed(source)
                    } else {
                        AssetLoadState::Ready(source)
                    };
                    self.states.insert(req.id, state);
                }
                AssetRequestKind::AtlasManifest => match self.load_atlas_manifest(&req.relative_path) {
                    Ok((atlas, source)) => {
                        self.loaded_atlases.insert(req.id.clone(), atlas);
                        self.states.insert(req.id, AssetLoadState::Ready(source));
                    }
                    Err(e) => {
                        self.states.insert(req.id, AssetLoadState::Failed(e.to_string()));
                    }
                },
                AssetRequestKind::RawBytes => match self.load_bytes(&req.relative_path) {
                    Ok((bytes, source)) => {
                        self.loaded_bytes.insert(req.id.clone(), bytes);
                        self.states.insert(req.id, AssetLoadState::Ready(source));
                    }
                    Err(e) => {
                        self.states.insert(req.id, AssetLoadState::Failed(e.to_string()));
                    }
                },
                AssetRequestKind::TextFile => match self.load_text(&req.relative_path) {
                    Ok((text, source)) => {
                        self.loaded_texts.insert(req.id.clone(), text);
                        self.states.insert(req.id, AssetLoadState::Ready(source));
                    }
                    Err(e) => {
                        self.states.insert(req.id, AssetLoadState::Failed(e.to_string()));
                    }
                },
            }
        }

        processed
    }

    /// Retorna o progresso normalizado (`0.0` a `1.0`) da fila de carregamento.
    pub fn load_progress(&self) -> f32 {
        if self.total_queued == 0 {
            1.0
        } else {
            (self.total_completed as f32 / self.total_queued as f32).clamp(0.0, 1.0)
        }
    }

    pub fn pending_count(&self) -> usize {
        self.load_queue.len()
    }

    pub fn state_of(&self, id: &str) -> Option<&AssetLoadState> {
        self.states.get(id)
    }

    pub fn get_texture(&self, id: &str) -> Option<&RgbaImage> {
        self.loaded_textures.get(id)
    }

    pub fn get_atlas(&self, id: &str) -> Option<&TextureAtlas> {
        self.loaded_atlases.get(id)
    }

    pub fn get_bytes(&self, id: &str) -> Option<&[u8]> {
        self.loaded_bytes.get(id).map(|v| v.as_slice())
    }

    pub fn get_text(&self, id: &str) -> Option<&str> {
        self.loaded_texts.get(id).map(|s| s.as_str())
    }

    /// Constrói o pacote completo de Atlas do Santuário (`512x512`):
    /// - Gera todas as regiões UV e ilustrações procedurais embutidas (grama isométrica, habitat,
    ///   fazenda, ninho, criaturas, moedas e frutas);
    /// - Garante que o bloco `(0..8, 0..8)` seja branco puro `[255, 255, 255, 255]` para `draw_colored_rect`;
    /// - Se existirem PNGs reais em `assets/sprites/<id>.png`, sobrepõe com a arte real do disco;
    /// - Embute os glifos da `BitmapFont` em `(0, 256)` para que cena, UI e texto compartilhem 1 único draw call!
    pub fn build_sanctuary_atlas_bundle(&self, font: &mut BitmapFont) -> SanctuaryAtlasBundle {
        let w = Self::EMBEDDED_ATLAS_WIDTH;
        let h = Self::EMBEDDED_ATLAS_HEIGHT;
        let mut atlas = TextureAtlas::new("sanctuary_atlas", w, h);
        let mut image = RgbaImage::solid(w, h, [0, 0, 0, 0]);

        // 1. Bloco branco puro no canto superior esquerdo (0, 0, 8, 8) para UI / draw_colored_rect
        atlas.add_region("white_pixel", Rect::new(0.0, 0.0, 8.0, 8.0));
        image.fill_rect(0, 0, 8, 8, [255, 255, 255, 255]);

        // 2. Regiões mapeadas do santuário dentro da folha 512x512
        let regions_spec: [(&str, u32, u32, u32, u32); 8] = [
            ("tile_island_grass_01", 0, 16, 128, 64),
            ("habitat_fire_t1", 128, 0, 128, 128),
            ("building_farm_empty", 256, 0, 96, 96),
            ("building_hatchery", 352, 0, 96, 96),
            ("creature_pyro_drake_baby", 0, 96, 64, 64),
            ("creature_volcano_beast_baby", 64, 96, 64, 64),
            ("icon_gold_coin", 0, 160, 32, 32),
            ("icon_food_apple", 32, 160, 32, 32),
        ];

        for &(id, rx, ry, rw, rh) in &regions_spec {
            atlas.add_region(id, Rect::new(rx as f32, ry as f32, rw as f32, rh as f32));
        }

        // 3. Desenha arte procedural embutida de fallback para cada região
        draw_embedded_isometric_grass(&mut image, 0, 16, 128, 64);
        draw_embedded_habitat(&mut image, 128, 0, 128, 128);
        draw_embedded_farm(&mut image, 256, 0, 96, 96);
        draw_embedded_hatchery(&mut image, 352, 0, 96, 96);
        draw_embedded_creature(&mut image, 0, 96, 64, 64, false);
        draw_embedded_creature(&mut image, 64, 96, 64, 64, true);
        draw_embedded_coin(&mut image, 0, 160, 32, 32);
        draw_embedded_apple(&mut image, 32, 160, 32, 32);

        // 4. Se houver sprites PNG reais na pasta assets/sprites/<id>.png, sobrepõe o fallback!
        let mut external_loaded = 0usize;
        for &(id, rx, ry, rw, rh) in &regions_spec {
            let candidate_path = format!("sprites/{}.png", id);
            if let Ok((bytes, _)) = self.load_bytes(&candidate_path) {
                if let Ok(sprite_img) = RgbaImage::from_png_bytes(&bytes) {
                    if sprite_img.width <= rw && sprite_img.height <= rh {
                        image.fill_rect(rx, ry, rw, rh, [0, 0, 0, 0]);
                        image.blit(&sprite_img, rx, ry);
                        external_loaded += 1;
                    }
                }
            }
        }

        // 5. Embute a BitmapFont em (0, 256) e remapeia seus glifos para a folha 512x512
        font.embed_into_atlas(&mut image, 0, 256);

        let source = if external_loaded > 0 {
            AssetSource::Filesystem(self.assets_root.clone())
        } else {
            AssetSource::Embedded("embedded_sanctuary_atlas_512x512".to_string())
        };

        SanctuaryAtlasBundle {
            atlas,
            image,
            source,
            external_sprites_loaded: external_loaded,
        }
    }
}

/// Desenha um losango isométrico 2:1 de grama verde com borda sombreada.
fn draw_embedded_isometric_grass(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32) {
    let hw = w as f32 * 0.5;
    let hh = h as f32 * 0.5;
    for y in 0..h {
        for x in 0..w {
            let dx = ((x as f32 + 0.5) - hw).abs() / hw;
            let dy = ((y as f32 + 0.5) - hh).abs() / hh;
            let diamond_dist = dx + dy;
            if diamond_dist <= 1.0 {
                let is_border = diamond_dist > 0.88;
                let color = if is_border {
                    [46, 118, 52, 255]
                } else if (x + y) % 7 == 0 {
                    [82, 178, 78, 255]
                } else {
                    [68, 158, 66, 255]
                };
                img.set_pixel(ox + x, oy + y, color);
            }
        }
    }
}

/// Desenha um santuário elemental circular com borda de rocha e núcleo brilhante.
fn draw_embedded_habitat(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.62;
    let rx = w as f32 * 0.44;
    let ry = h as f32 * 0.30;

    for y in 0..h {
        for x in 0..w {
            let nx = (x as f32 - cx) / rx;
            let ny = (y as f32 - cy) / ry;
            let d2 = nx * nx + ny * ny;
            if d2 <= 1.0 {
                let color = if d2 > 0.72 {
                    [110, 95, 90, 255] // Anel rochoso externo
                } else if d2 > 0.30 {
                    [185, 85, 45, 255] // Solo vulcânico quente
                } else {
                    [245, 165, 55, 255] // Núcleo energético elemental
                };
                img.set_pixel(ox + x, oy + y, color);
            }
        }
    }
}

/// Desenha um canteiro agrícola com sulcos de plantio.
fn draw_embedded_farm(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32) {
    let margin = 10;
    for y in margin..(h - margin) {
        for x in margin..(w - margin) {
            let is_border = x < margin + 4 || x >= w - margin - 4 || y < margin + 4 || y >= h - margin - 4;
            let is_furrow = (y / 8) % 2 == 0;
            let color = if is_border {
                [135, 90, 48, 255] // Moldura de madeira
            } else if is_furrow {
                [98, 64, 34, 255] // Terra arada escura
            } else {
                [76, 152, 58, 255] // Brotos verdes
            };
            img.set_pixel(ox + x, oy + y, color);
        }
    }
}

/// Desenha o Ninho de Incubação trançado com ovos místicos no centro.
fn draw_embedded_hatchery(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.58;
    let r = w as f32 * 0.40;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / r;
            if dist <= 1.0 {
                let color = if dist > 0.62 {
                    [198, 148, 68, 255] // Palha dourada trançada
                } else if dist > 0.35 {
                    [142, 98, 44, 255] // Interior acolhedor do ninho
                } else {
                    [248, 238, 210, 255] // Ovo luminoso no centro
                };
                img.set_pixel(ox + x, oy + y, color);
            }
        }
    }
}

/// Desenha uma criatura estilizada em tons claros (para suportar tinting elemental dinâmico).
fn draw_embedded_creature(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32, horned: bool) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.56;
    let r = w as f32 * 0.34;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / r;
            let horn = horned && y < h / 3 && (dx.abs() > 6.0 && dx.abs() < 14.0);
            if dist <= 1.0 || horn {
                let is_eye = (dx.abs() - 6.0).abs() < 2.5 && (dy + 4.0).abs() < 2.5;
                let color = if is_eye {
                    [25, 25, 35, 255]
                } else if dist > 0.82 {
                    [180, 180, 195, 255] // Contorno suave
                } else {
                    [245, 245, 255, 255] // Corpo claro modulável por tint_color
                };
                img.set_pixel(ox + x, oy + y, color);
            }
        }
    }
}

/// Desenha ícone de moeda dourada.
fn draw_embedded_coin(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let r = w as f32 * 0.42;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt() / r;
            if d <= 1.0 {
                let color = if d > 0.78 {
                    [205, 145, 20, 255]
                } else {
                    [255, 215, 55, 255]
                };
                img.set_pixel(ox + x, oy + y, color);
            }
        }
    }
}

/// Desenha ícone de maçã/comida.
fn draw_embedded_apple(img: &mut RgbaImage, ox: u32, oy: u32, w: u32, h: u32) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.55;
    let r = w as f32 * 0.38;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt() / r;
            if d <= 1.0 {
                img.set_pixel(ox + x, oy + y, [225, 55, 50, 255]);
            } else if y >= 3 && y <= 8 && (x as i32 - (w as i32 / 2)).abs() <= 2 {
                img.set_pixel(ox + x, oy + y, [75, 185, 65, 255]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_loader_embedded_and_filesystem_priority() {
        let temp_dir = std::env::temp_dir().join("bestigard_asset_loader_test");
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("config.txt");
        fs::write(&file_path, b"disk_content").unwrap();

        let mut loader = AssetLoader::new(&temp_dir);

        // 1. Leitura a partir do filesystem
        let (text, source) = loader.load_text("config.txt").unwrap();
        assert_eq!(text, "disk_content");
        assert!(matches!(source, AssetSource::Filesystem(_)));

        // 2. Registro de blob embutido tem prioridade imediata (modo APK / embedded)
        loader.register_embedded_bytes("config.txt", b"embedded_override".to_vec());
        let (text2, source2) = loader.load_text("config.txt").unwrap();
        assert_eq!(text2, "embedded_override");
        assert_eq!(source2, AssetSource::Embedded("config.txt".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_async_load_queue_progression() {
        let mut loader = AssetLoader::for_current_platform();
        loader.register_embedded_bytes("data/hello.txt", b"Bestigard".to_vec());

        loader.enqueue("txt_hello", "data/hello.txt", AssetRequestKind::TextFile);
        loader.enqueue(
            "tex_missing",
            "sprites/non_existent.png",
            AssetRequestKind::TexturePng {
                fallback_w: 32,
                fallback_h: 32,
            },
        );

        assert_eq!(loader.pending_count(), 2);
        assert_eq!(loader.load_progress(), 0.0);

        // Processa 1 item por frame
        assert_eq!(loader.poll_load_queue(1), 1);
        assert_eq!(loader.pending_count(), 1);
        assert!((loader.load_progress() - 0.5).abs() < 1e-4);
        assert!(matches!(
            loader.state_of("txt_hello"),
            Some(AssetLoadState::Ready(AssetSource::Embedded(_)))
        ));

        // Processa o 2º item (usa fallback automático para PNG ausente)
        assert_eq!(loader.poll_load_queue(1), 1);
        assert_eq!(loader.pending_count(), 0);
        assert!((loader.load_progress() - 1.0).abs() < 1e-4);
        assert!(matches!(
            loader.state_of("tex_missing"),
            Some(AssetLoadState::FallbackUsed(_))
        ));
        assert!(loader.get_texture("tex_missing").is_some());
    }

    #[test]
    fn test_embedded_sanctuary_atlas_bundle_and_font_remapping() {
        let loader = AssetLoader::for_current_platform();
        let mut font = BitmapFont::default_font();
        let bundle = loader.build_sanctuary_atlas_bundle(&mut font);

        assert_eq!(bundle.image.width, 512);
        assert_eq!(bundle.image.height, 512);
        assert!(bundle.atlas.contains("white_pixel"));
        assert!(bundle.atlas.contains("tile_island_grass_01"));
        assert!(bundle.atlas.contains("habitat_fire_t1"));
        assert!(bundle.atlas.contains("building_farm_empty"));
        assert!(bundle.atlas.contains("building_hatchery"));
        assert!(bundle.atlas.contains("creature_pyro_drake_baby"));
        assert!(bundle.atlas.contains("creature_volcano_beast_baby"));
        assert!(bundle.atlas.contains("icon_gold_coin"));
        assert!(bundle.atlas.contains("icon_food_apple"));

        // Canto (0, 0) deve ser branco puro para suportar draw_colored_rect
        assert_eq!(bundle.image.get_pixel(0, 0), Some([255, 255, 255, 255]));

        // Centro do tile de grama (64, 48) deve estar preenchido (alpha = 255)
        let grass_px = bundle.image.get_pixel(64, 48).unwrap();
        assert_eq!(grass_px[3], 255);

        // Glifos da BitmapFont devem ter sido remapeados para o offset Y = 256 (v >= 0.5)
        let glyph_a = font.glyphs.get(&'A').unwrap();
        assert!(glyph_a.uv_min.1 >= 0.5);
    }
}

