//! Sistema elemental desacoplado e orientado a dados.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identificador único para um elemento (ex: "fire", "water", "shadow").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ElementId(pub String);

impl ElementId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into().to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ElementId {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadados e propriedades visuais/temáticas de um elemento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    pub id: ElementId,
    pub name: String,
    pub color_hex: String,
    pub description: String,
}

impl Element {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<String>,
        color_hex: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color_hex: color_hex.into(),
            description: description.into(),
        }
    }
}

/// Registro configurável de elementos conhecidos no jogo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementRegistry {
    elements: HashMap<ElementId, Element>,
}

impl ElementRegistry {
    /// Cria um registro vazio.
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
        }
    }

    /// Cria o registro pré-preenchido com os 7 elementos fundamentais de Bestigard.
    pub fn with_default_elements() -> Self {
        let mut registry = Self::new();

        registry.register(Element::new(
            "fire",
            "Fogo",
            "#FF4500",
            "Poder térmico e chamas ardentes da destruição e renascimento.",
        ));
        registry.register(Element::new(
            "water",
            "Água",
            "#1E90FF",
            "Fluidez, cura e torrentes místicas das profundezas oceânicas.",
        ));
        registry.register(Element::new(
            "air",
            "Ar",
            "#87CEEB",
            "Ventos velozes, altitude e correntes puras da atmosfera.",
        ));
        registry.register(Element::new(
            "earth",
            "Terra",
            "#8B4513",
            "Rocha inabalável, fertilidade e força tectônica primitiva.",
        ));
        registry.register(Element::new(
            "lightning",
            "Raio",
            "#FFD700",
            "Energia eletrizante de alta voltagem e tempestades repentinas.",
        ));
        registry.register(Element::new(
            "light",
            "Luz",
            "#FFFACD",
            "Radiância sagrada, clareza celestial e éter primordial.",
        ));
        registry.register(Element::new(
            "shadow",
            "Sombra",
            "#4B0082",
            "Mistérios do vazio, ilusões arcanas e forças cósmicas obscuras.",
        ));

        registry
    }

    /// Registra ou substitui a definição de um elemento.
    pub fn register(&mut self, element: Element) {
        self.elements.insert(element.id.clone(), element);
    }

    /// Remove um elemento pelo ID (facilitando rebalanceamentos futuros).
    pub fn remove(&mut self, id: &ElementId) -> Option<Element> {
        self.elements.remove(id)
    }

    /// Retorna uma referência a um elemento registrado.
    pub fn get(&self, id: &ElementId) -> Option<&Element> {
        self.elements.get(id)
    }

    /// Verifica se um elemento existe no registro.
    pub fn contains(&self, id: &ElementId) -> bool {
        self.elements.contains_key(id)
    }

    /// Retorna a lista de todos os elementos cadastrados.
    pub fn all(&self) -> impl Iterator<Item = &Element> {
        self.elements.values()
    }

    /// Retorna a quantidade total de elementos registrados.
    pub fn count(&self) -> usize {
        self.elements.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_elements_registration() {
        let registry = ElementRegistry::with_default_elements();

        assert_eq!(registry.count(), 7);
        assert!(registry.contains(&ElementId::new("fire")));
        assert!(registry.contains(&ElementId::new("water")));
        assert!(registry.contains(&ElementId::new("air")));
        assert!(registry.contains(&ElementId::new("earth")));
        assert!(registry.contains(&ElementId::new("lightning")));
        assert!(registry.contains(&ElementId::new("light")));
        assert!(registry.contains(&ElementId::new("shadow")));
    }

    #[test]
    fn test_dynamic_add_and_remove() {
        let mut registry = ElementRegistry::new();
        assert_eq!(registry.count(), 0);

        // Adiciona elemento customizado
        let techno = Element::new("techno", "Tecnomagia", "#00FFFF", "Circuitos arcanos");
        registry.register(techno);
        assert_eq!(registry.count(), 1);
        assert!(registry.contains(&ElementId::new("techno")));

        // Remove elemento facilmente
        registry.remove(&ElementId::new("techno"));
        assert_eq!(registry.count(), 0);
        assert!(!registry.contains(&ElementId::new("techno")));
    }
}
