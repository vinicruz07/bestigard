//! Gerenciador de Atlas de Texturas e mapeamento de coordenadas UV em Safe Rust.
#![forbid(unsafe_code)]

use crate::geometry::Rect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Região mapeada de um sprite individual dentro de um atlas de textura.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasRegion {
    /// Identificador único do sprite (ex: "creature_pyro_drake_baby", "tile_island_grass_01").
    pub id: String,
    /// Bounding box em pixels dentro da imagem do atlas.
    pub frame: Rect,
    /// Coordenada UV normalizada superior-esquerda (U mínimo).
    pub u_min: f32,
    /// Coordenada UV normalizada superior-esquerda (V mínimo).
    pub v_min: f32,
    /// Coordenada UV normalizada inferior-direita (U máximo).
    pub u_max: f32,
    /// Coordenada UV normalizada inferior-direita (V máximo).
    pub v_max: f32,
    /// Largura original do asset não-aparado em pixels.
    pub orig_width: f32,
    /// Altura original do asset não-aparado em pixels.
    pub orig_height: f32,
}

impl AtlasRegion {
    pub fn new(id: impl Into<String>, frame: Rect, tex_width: f32, tex_height: f32) -> Self {
        let (u_min, v_min, u_max, v_max) = frame.uv_normalized(tex_width, tex_height);
        Self {
            id: id.into(),
            orig_width: frame.w,
            orig_height: frame.h,
            frame,
            u_min,
            v_min,
            u_max,
            v_max,
        }
    }

    /// Retorna as 4 coordenadas UV ordenadas para um quad:
    /// [Top-Left, Top-Right, Bottom-Right, Bottom-Left].
    pub fn quad_uvs(&self, flip_x: bool, flip_y: bool) -> [[f32; 2]; 4] {
        let (u0, u1) = if flip_x {
            (self.u_max, self.u_min)
        } else {
            (self.u_min, self.u_max)
        };

        let (v0, v1) = if flip_y {
            (self.v_max, self.v_min)
        } else {
            (self.v_min, self.v_max)
        };

        [
            [u0, v0], // Top-Left
            [u1, v0], // Top-Right
            [u1, v1], // Bottom-Right
            [u0, v1], // Bottom-Left
        ]
    }
}

/// Formato serializável para intercâmbio de atlas em JSON (ex: exportado por TexturePacker/Aseprite).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasManifestDef {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub regions: Vec<AtlasRegionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasRegionDef {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Dicionário de regiões de sprites indexadas pertencentes a uma mesma folha/textura de atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureAtlas {
    pub name: String,
    pub width: u32,
    pub height: u32,
    regions: HashMap<String, AtlasRegion>,
}

impl TextureAtlas {
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            regions: HashMap::new(),
        }
    }

    /// Registra uma nova região de sprite fornecendo coordenadas em pixels.
    pub fn add_region(&mut self, id: impl Into<String>, frame: Rect) -> &AtlasRegion {
        let region = AtlasRegion::new(id, frame, self.width as f32, self.height as f32);
        let id_key = region.id.clone();
        self.regions.insert(id_key.clone(), region);
        self.regions.get(&id_key).unwrap()
    }

    /// Obtém referência à região do sprite pelo identificador.
    pub fn get(&self, id: &str) -> Option<&AtlasRegion> {
        self.regions.get(id)
    }

    /// Obtém a região solicitada ou retorna uma região padrão de fallback se fornecida.
    pub fn get_or_fallback<'a>(&'a self, id: &str, fallback_id: &'a str) -> Option<&'a AtlasRegion> {
        self.get(id).or_else(|| self.get(fallback_id))
    }

    /// Verifica se o sprite está cadastrado neste atlas.
    pub fn contains(&self, id: &str) -> bool {
        self.regions.contains_key(id)
    }

    /// Quantidade total de sprites mapeados.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Itera sobre todas as regiões de sprites do atlas.
    pub fn iter(&self) -> impl Iterator<Item = &AtlasRegion> {
        self.regions.values()
    }

    /// Exporta a definição do atlas em JSON legível.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Importa a definição do atlas a partir de JSON.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        serde_json::from_str(json_str).map_err(|e| e.to_string())
    }

    /// Constrói um atlas a partir de um manifesto estruturado.
    pub fn from_manifest(manifest: AtlasManifestDef) -> Self {
        let mut atlas = Self::new(manifest.name, manifest.width, manifest.height);
        for reg in manifest.regions {
            atlas.add_region(reg.id, Rect::new(reg.x, reg.y, reg.w, reg.h));
        }
        atlas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_registration_and_uv_bounds() {
        let mut atlas = TextureAtlas::new("test_sheet", 1024, 1024);
        atlas.add_region("pyro_drake_baby", Rect::new(0.0, 0.0, 128.0, 128.0));
        atlas.add_region("aqua_serpent_baby", Rect::new(128.0, 0.0, 128.0, 128.0));

        assert_eq!(atlas.len(), 2);
        assert!(atlas.contains("pyro_drake_baby"));

        let drake = atlas.get("pyro_drake_baby").expect("Region must exist");
        assert_eq!(drake.u_min, 0.0);
        assert_eq!(drake.v_min, 0.0);
        assert_eq!(drake.u_max, 0.125);
        assert_eq!(drake.v_max, 0.125);

        let uvs = drake.quad_uvs(false, false);
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [0.125, 0.0]);
        assert_eq!(uvs[2], [0.125, 0.125]);
        assert_eq!(uvs[3], [0.0, 0.125]);

        let flipped_uvs = drake.quad_uvs(true, false);
        assert_eq!(flipped_uvs[0], [0.125, 0.0]);
        assert_eq!(flipped_uvs[1], [0.0, 0.0]);
    }

    #[test]
    fn test_atlas_json_roundtrip() {
        let mut atlas = TextureAtlas::new("creatures_atlas", 2048, 2048);
        atlas.add_region("egg_pyro", Rect::new(10.0, 10.0, 64.0, 64.0));

        let json = atlas.to_json().expect("Serialization should succeed");
        let deserialized = TextureAtlas::from_json(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.name, "creatures_atlas");
        assert_eq!(deserialized.width, 2048);
        assert_eq!(deserialized.len(), 1);
        assert!(deserialized.contains("egg_pyro"));
    }
}
