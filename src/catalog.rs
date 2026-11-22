//! Warenkatalog von KAUF NULL (tools/catalog.json, beim Bauen eingebettet).

use crate::assets::{half_extent_idx, CATALOG_JSON, FIXED_COUNT, PRODUCT_MESH_IDS};
use glam::Vec3;
use serde::Deserialize;
use std::sync::OnceLock;

/// Welche Art Einrichtung eine Unterabteilung normalerweise hat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FixtureKind {
    Produce,
    Shelf,
    Cooler,
    Freezer,
}

#[derive(Deserialize)]
struct RawSub {
    id: String,
    name: String,
    dept: String,
    fixture: FixtureKind,
}

#[derive(Deserialize)]
struct RawProduct {
    mesh: String,
    sub: String,
    name: String,
    cents: u32,
    #[serde(default)]
    list: Option<String>,
    #[serde(default)]
    pricey: bool,
    #[serde(default)]
    tint: Option<[f32; 3]>,
}

#[derive(Deserialize)]
struct RawCatalog {
    subs: Vec<RawSub>,
    products: Vec<RawProduct>,
}

pub struct Sub {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub dept: String,
    pub fixture: FixtureKind,
    /// Produkte dieser Unterabteilung
    pub products: Vec<usize>,
}

pub struct Product {
    pub name: String,
    pub cents: u32,
    pub sub: usize,
    /// globaler Modell-Index (fuer Renderer und MeshDraw)
    pub mesh: usize,
    pub tint: Vec3,
    /// Index in `Catalog::list`, falls das Produkt auf dem Einkaufszettel stehen kann
    pub list: Option<usize>,
    pub pricey: bool,
    /// halbe Groesse des Modells
    pub half: Vec3,
}

/// Ein moeglicher Zettel-Eintrag mit billigen und teuren Varianten.
pub struct ListEntry {
    pub name: String,
    pub cheap: Vec<usize>,
    pub pricey: Vec<usize>,
}

pub struct Catalog {
    pub subs: Vec<Sub>,
    pub products: Vec<Product>,
    pub list: Vec<ListEntry>,
}

impl Catalog {
    pub fn sub_index(&self, id: &str) -> usize {
        self.subs
            .iter()
            .position(|s| s.id == id)
            .unwrap_or_else(|| panic!("Unterabteilung {id} fehlt"))
    }
    pub fn list_index(&self, name: &str) -> Option<usize> {
        self.list.iter().position(|l| l.name == name)
    }
}

pub fn catalog() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(|| {
        let raw: RawCatalog = serde_json::from_str(CATALOG_JSON).expect("catalog.json ungueltig");
        let mut subs: Vec<Sub> = raw
            .subs
            .into_iter()
            .map(|s| Sub {
                id: s.id,
                name: s.name,
                dept: s.dept,
                fixture: s.fixture,
                products: Vec::new(),
            })
            .collect();
        let mut list: Vec<ListEntry> = Vec::new();
        let mut products = Vec::new();
        for p in raw.products {
            let sub = subs
                .iter()
                .position(|s| s.id == p.sub)
                .unwrap_or_else(|| panic!("Unterabteilung {} fehlt", p.sub));
            let mesh_local = PRODUCT_MESH_IDS
                .iter()
                .position(|m| *m == p.mesh)
                .unwrap_or_else(|| panic!("Modell {} fehlt", p.mesh));
            let mesh = FIXED_COUNT + mesh_local;
            let list_idx =
                p.list
                    .as_ref()
                    .map(|name| match list.iter().position(|l| &l.name == name) {
                        Some(i) => i,
                        None => {
                            list.push(ListEntry {
                                name: name.clone(),
                                cheap: Vec::new(),
                                pricey: Vec::new(),
                            });
                            list.len() - 1
                        }
                    });
            let idx = products.len();
            if let Some(l) = list_idx {
                if p.pricey {
                    list[l].pricey.push(idx);
                } else {
                    list[l].cheap.push(idx);
                }
            }
            subs[sub].products.push(idx);
            products.push(Product {
                name: p.name,
                cents: p.cents,
                sub,
                mesh,
                tint: p.tint.map(Vec3::from).unwrap_or(Vec3::ONE),
                list: list_idx,
                pricey: p.pricey,
                half: half_extent_idx(mesh),
            });
        }
        Catalog {
            subs,
            products,
            list,
        }
    })
}

/// Falsch beschriftete Variante eines Namens ("Milch" -> "Mlich"): zwei innere Buchstaben getauscht.
pub fn misspell(name: &str, seed: u64) -> String {
    let mut chars: Vec<char> = name.chars().collect();
    let letters: Vec<usize> = (1..chars.len().saturating_sub(1))
        .filter(|&i| chars[i].is_alphabetic() && chars[i + 1].is_alphabetic())
        .collect();
    if letters.is_empty() {
        return format!("{name}?");
    }
    let i = letters[(seed as usize) % letters.len()];
    chars.swap(i, i + 1);
    let s: String = chars.into_iter().collect();
    if s == name {
        format!("{name}0")
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_loads_and_list_entries_have_both_variants() {
        let c = catalog();
        assert!(c.products.len() > 100);
        assert!(c.list.len() >= 18);
        for l in &c.list {
            assert!(
                !l.cheap.is_empty() && !l.pricey.is_empty(),
                "{} braucht billig + teuer",
                l.name
            );
            let cheap = l.cheap.iter().map(|&p| c.products[p].cents).max().unwrap();
            let pricey = l.pricey.iter().map(|&p| c.products[p].cents).min().unwrap();
            assert!(cheap < pricey, "{}: billig muss billiger sein", l.name);
        }
        for s in &c.subs {
            assert!(!s.products.is_empty(), "{} leer", s.id);
        }
        assert!(c.list_index("Mineralwasser").is_some());
    }

    #[test]
    fn misspelling_changes_name() {
        for n in ["Milch", "Käse", "Eis", "Klopapier 8 Rollen"] {
            assert_ne!(misspell(n, 3), n);
        }
    }
}
