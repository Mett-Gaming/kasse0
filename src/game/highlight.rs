//! Passende Ware in Griffnaehe leuchtet auf: immer nur eine, die naechste im Umkreis von 1 m.
//! Passend ist, was zu einem offenen Zettel-Eintrag gehoert, billig oder teuer, und auch die
//! falsch beschriftete Ausgabe davon: Die Hervorhebung verraet nicht, ob die Ware echt ist.

use super::*;

/// Umkreis um den Spieler (Meter, in der Ebene)
pub const HIGHLIGHT_RADIUS: f32 = 1.0;
/// Eine andere Ware muss um so viel naeher sein, damit die Hervorhebung wechselt
const SWITCH_MARGIN: f32 = 0.05;
/// So tief im Regal darf die Ware stehen (weiter hinten ist die andere Regalseite)
const SHELF_DEPTH: f32 = 0.4;

/// Welche Ware leuchtet: im Regal oder am Boden liegend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Highlight {
    Shelf(usize),
    Loose(usize),
}

impl Game {
    /// Gehoert diese Ware (echt oder falsch beschriftet) zu einem offenen Zettel-Eintrag?
    fn could_fit_list(&self, product: usize) -> bool {
        catalog().products[product]
            .list
            .is_some_and(|l| self.list.contains(&l) && !self.list_done(l))
    }

    /// Hervorgehobene Ware neu bestimmen (jedes Bild).
    pub(super) fn update_highlight(&mut self) {
        let old = self.highlight.take();
        let busy = self.held.is_some()
            || self.paid
            || self.dying.is_some()
            || self.dialog.is_some()
            || self.checking_out()
            || self.till_paying()
            || self.outcome != Outcome::Running;
        if busy {
            return;
        }
        let eye = self.eye();
        let player = self.player.pos;
        let in_reach =
            |p: Vec3| vec3(p.x - player.x, 0.0, p.z - player.z).length() <= HIGHLIGHT_RADIUS;
        let mut found: Vec<(f32, Highlight, Vec3)> = Vec::new();
        for i in self.world.items_near(player, HIGHLIGHT_RADIUS) {
            let it = &self.world.items[i];
            if !it.taken && in_reach(it.pos) && self.could_fit_list(it.product as usize) {
                let c = it.pos + vec3(0.0, it.half().y, 0.0);
                found.push(((c - eye).length(), Highlight::Shelf(i), c));
            }
        }
        for (i, l) in self.loose.iter().enumerate() {
            if let PickupKind::Product { product, .. } = l.data.kind {
                if in_reach(l.pos) && self.could_fit_list(product) {
                    let c = l.pos + vec3(0.0, l.data.half().y, 0.0);
                    found.push(((c - eye).length(), Highlight::Loose(i), c));
                }
            }
        }
        // die bisherige Ware bekommt einen kleinen Vorsprung, sonst flackert es zwischen zweien
        for f in &mut found {
            if Some(f.1) == old {
                f.0 -= SWITCH_MARGIN;
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.highlight = found
            .into_iter()
            .find(|&(_, _, c)| {
                // nicht durch Waende oder auf die andere Regalseite
                let d = c - eye;
                let len = d.length();
                len <= SHELF_DEPTH
                    || self
                        .world
                        .line_of_sight(eye, eye + d * ((len - SHELF_DEPTH) / len))
            })
            .map(|f| f.1);
    }

    /// Die hervorgehobene Ware noch einmal darueberzeichnen: etwas groesser, pulsierend hell.
    pub(super) fn collect_highlight_draw(&self, out: &mut Vec<MeshDraw>) {
        if self.dying.is_some() {
            return;
        }
        let (mut d, half_y) = match self.highlight {
            Some(Highlight::Shelf(i)) => {
                let Some(it) = self.world.items.get(i).filter(|it| !it.taken) else {
                    return;
                };
                let mut d = it.draw();
                d.model = Mat4::from_translation(self.sway_at(it.pos)) * d.model;
                (d, it.half().y)
            }
            Some(Highlight::Loose(i)) => {
                let Some(l) = self.loose.get(i) else { return };
                let d = MeshDraw {
                    mesh: l.data.mesh,
                    model: Mat4::from_rotation_translation(
                        Quat::from_rotation_y(-l.yaw),
                        l.pos + vec3(0.0, self.sway_at(l.pos).y, 0.0),
                    ),
                    tint: l.data.tint.extend(0.0),
                };
                (d, l.data.half().y)
            }
            None => return,
        };
        let pulse = 0.5 + 0.5 * (self.time * 5.0).sin();
        let up = vec3(0.0, half_y, 0.0);
        d.model = d.model
            * Mat4::from_translation(up)
            * Mat4::from_scale(Vec3::splat(1.06 + 0.04 * pulse))
            * Mat4::from_translation(-up);
        d.tint = (d.tint.truncate() * 1.15).extend(0.45 + 0.45 * pulse);
        out.push(d);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::shelf_item;
    use super::*;

    /// Spieler `d` Meter neben die Ware stellen, bis etwas leuchtet.
    fn stand_at(g: &mut Game, i: usize, d: f32) -> bool {
        let t = g.world.items[i].pos;
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let want = vec3(t.x + a.sin() * d, 0.0, t.z + a.cos() * d);
            if g.world.collide(want, 0.25).distance(want) < 1e-3 {
                g.player.pos = want;
                g.update_highlight();
                if g.highlight.is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// Spieler vor einer Ware mit offenem Zettel-Eintrag.
    fn shopping() -> Game {
        let mut g = Game::new(12);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        for e in g.list.clone() {
            if g.list_done(e) {
                continue;
            }
            for pricey in [false, true] {
                if let Some(i) = shelf_item(&g, e, pricey) {
                    if stand_at(&mut g, i, 0.8) {
                        return g;
                    }
                }
            }
        }
        panic!("keine erreichbare Zettel-Ware");
    }

    fn lit(g: &Game) -> usize {
        match g.highlight {
            Some(Highlight::Shelf(i)) => i,
            other => panic!("erwartet Regalware, war {other:?}"),
        }
    }

    #[test]
    fn the_nearest_matching_item_lights_up_and_only_one() {
        let g = shopping();
        let it = &g.world.items[lit(&g)];
        assert!(g.could_fit_list(it.product as usize));
        let flat = vec3(it.pos.x - g.player.pos.x, 0.0, it.pos.z - g.player.pos.z);
        assert!(flat.length() <= HIGHLIGHT_RADIUS);
        let mut out = Vec::new();
        g.collect_highlight_draw(&mut out);
        assert_eq!(out.len(), 1, "genau eine Ware");
        assert!(out[0].tint.w > 0.3, "leuchtet nicht");
    }

    #[test]
    fn nothing_lights_up_out_of_reach_when_done_or_with_full_hands() {
        let mut g = shopping();
        let i = lit(&g);
        let entry = catalog().products[g.world.items[i].product as usize].list;
        g.held = Some(ItemData {
            kind: PickupKind::Product {
                product: g.world.items[i].product as usize,
                fake: false,
            },
            ..ItemData::from_shelf(&g.world.items[i])
        });
        g.update_highlight();
        assert_eq!(g.highlight, None, "leuchtet mit vollen Haenden");
        // Eintrag erledigt: diese Ware leuchtet nicht mehr
        let done = g.held.take().unwrap();
        g.cart.push(done);
        g.update_highlight();
        if let Some(Highlight::Shelf(k)) = g.highlight {
            let p = g.world.items[k].product as usize;
            assert_ne!(catalog().products[p].list, entry);
        }
        // weit weg von allen Regalen
        g.cart.clear();
        g.player.pos = g.world.spawn;
        g.update_highlight();
        assert_eq!(g.highlight, None, "am Eingang leuchtet etwas");
    }

    #[test]
    fn mislabeled_items_light_up_just_like_real_ones() {
        let mut g = shopping();
        let i = lit(&g);
        for fake in [true, false] {
            g.world.items[i].fake = fake;
            g.update_highlight();
            assert_eq!(g.highlight, Some(Highlight::Shelf(i)), "verraet sich");
        }
    }

    #[test]
    fn dropped_matching_items_light_up_and_closer_ones_win() {
        let mut g = shopping();
        let data = ItemData::from_shelf(&g.world.items[lit(&g)]);
        let pos = g.eye() + g.view_dir() * 0.1;
        g.loose.push(Loose {
            data,
            pos,
            vel: Vec3::ZERO,
            yaw: 0.0,
        });
        g.update_highlight();
        assert_eq!(g.highlight, Some(Highlight::Loose(0)));
    }
}
