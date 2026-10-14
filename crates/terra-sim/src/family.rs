//! Families (M2 design §15.6): who's whose, what a sprite senses of its
//! family, and where a parent last saw its egg. The world gives the senses;
//! the genes say how anxious they make it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::courtship::{self, Company};
use crate::data::DataPack;
use crate::genome::Whom;
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::Target;
use crate::sprites::Sprite;
use crate::world::WorldState;

/// An egg of a sprite's own that it has seen (M2 design §15.6): where it
/// last saw it, and whether it saw no other parent by it then, so it left
/// it alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OwnEgg {
    pub(crate) egg: EntityId,
    pub(crate) at: Pos,
    pub(crate) alone: bool,
}

/// What a sprite senses of its family this tick (M2 design §15.6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FamilySenses {
    /// An egg of its own is out of sight.
    pub(crate) egg_away: bool,
    /// One of those it left with no other parent by it when it last saw
    /// it, or has never seen.
    pub(crate) egg_alone: bool,
    /// A young one of its own is out of sight.
    pub(crate) young_away: bool,
    /// It's young, and has a living parent, but none in sight.
    pub(crate) parent_away: bool,
    /// Its partner is out of sight.
    pub(crate) partner_away: bool,
}

/// Who a thing is to a sprite (M2 design §15.6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Relation {
    /// Its family: its partner, its parents while it's young, its young,
    /// or an egg of its own.
    pub(crate) family: bool,
    /// Its partner.
    pub(crate) partner: bool,
    /// Its parent, while it's young.
    pub(crate) parent: bool,
    /// Its young: a child not grown up.
    pub(crate) young: bool,
    /// Its close kin (§15.5): a parent, a young one or an egg of its own, or
    /// a brother or sister, hatched or not.
    pub(crate) kin: bool,
}

impl Relation {
    /// Whether an attention instinct about `whom` is about it.
    pub(crate) fn is(self, whom: Whom) -> bool {
        match whom {
            Whom::Any => true,
            Whom::Family => self.family,
            Whom::Kin => self.kin,
            Whom::Partner => self.partner,
            Whom::Parent => self.parent,
            Whom::Young => self.young,
        }
    }
}

/// Who `target` is to sprite `me` (M2 design §15.6).
pub(crate) fn relation(state: &WorldState, me: EntityId, target: Target) -> Relation {
    let Some(sprite) = state.sprites.get(me) else {
        return Relation::default();
    };
    match target {
        Target::Sprite(other) => {
            let Some(them) = state.sprites.get(other) else {
                return Relation::default();
            };
            let young = |sprite: &Sprite| !sprite.stage().is_grown();
            let partner = sprite.partner() == Some(other);
            let parent = young(sprite) && sprite.parents.contains(&other);
            let child = young(them) && them.parents.contains(&me);
            Relation {
                family: partner || parent || child,
                partner,
                parent,
                young: child,
                kin: courtship::kin(state, me, other),
            }
        }
        Target::Object(id) => {
            let egg = state.objects.get(id).and_then(|o| o.unhatched.as_deref());
            let Some(egg) = egg else {
                return Relation::default();
            };
            let family = egg.parents.contains(&me);
            let siblings = egg.parents.iter().any(|p| sprite.parents.contains(p));
            Relation {
                family,
                kin: family || siblings,
                ..Relation::default()
            }
        }
        Target::Water(_) | Target::Cursor => Relation::default(),
    }
}

/// Step 3's look for family, after the look round (M2 design §15.6): each
/// sprite notes where it sees each egg of its own, and whether it sees
/// another parent by it, and forgets one that has hatched or is gone; then
/// its `company` gains what it senses of its family. Nothing in a pack
/// without families.
pub(crate) fn look_for_family(
    state: &mut WorldState,
    data: &DataPack,
    company: &mut BTreeMap<EntityId, Company>,
) {
    let Some(indices) = data.physiology().indices.families else {
        return;
    };
    // Every egg, with its tile, none while the Cursor holds it, and its
    // parents; and the young sprites of each sprite.
    let eggs: Vec<(EntityId, Option<Pos>, Vec<EntityId>)> = if state.objects.eggs() == 0 {
        Vec::new()
    } else {
        let eggs = state.objects.iter().filter_map(|(id, object)| {
            let unhatched = object.unhatched.as_deref()?;
            let pos = (!object.held).then_some(object.pos);
            Some((id, pos, unhatched.parents.clone()))
        });
        eggs.collect()
    };
    let sprites = &state.sprites;
    let mut young: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    for (id, sprite) in sprites.iter() {
        if !sprite.stage().is_grown() {
            for &parent in &sprite.parents {
                young.entry(parent).or_default().push(id);
            }
        }
    }
    let mut seen_eggs: Vec<(EntityId, Vec<OwnEgg>)> = Vec::new();
    let mut found = Vec::new();
    for (id, sprite) in sprites.iter() {
        let company = company.get_mut(&id).expect("each sprite looked round");
        let sees = |other: EntityId| company.seen.binary_search(&other).is_ok();
        let mut senses = FamilySenses::default();
        let mut seen = Vec::new();
        for (egg, pos, parents) in eggs.iter().filter(|(_, _, p)| p.contains(&id)) {
            match pos.filter(|&pos| sprite.sees(pos)) {
                Some(at) => {
                    // By it: a parent it sees, which sees the egg.
                    let by_it = |p: EntityId| sees(p) && sprites.get(p).is_some_and(|o| o.sees(at));
                    let alone = !parents.iter().any(|&p| p != id && by_it(p));
                    seen.push(OwnEgg {
                        egg: *egg,
                        at,
                        alone,
                    });
                }
                None => {
                    senses.egg_away = true;
                    let known = sprite.eggs.iter().find(|known| known.egg == *egg);
                    senses.egg_alone |= known.is_none_or(|known| known.alone);
                }
            }
        }
        let young = young.get(&id);
        senses.young_away = young.is_some_and(|young| young.iter().any(|&y| !sees(y)));
        // Only living sprites are in sight.
        let has_parent = sprite.parents.iter().any(|&p| sprites.get(p).is_some());
        let sees_parent = sprite.parents.iter().any(|&p| sees(p));
        let is_young = !sprite.stage().is_grown();
        senses.parent_away = is_young && has_parent && !sees_parent;
        senses.partner_away = sprite.partner().is_some_and(|p| !sees(p));
        // It found family it was missing, with a `found_family` pulse: a
        // sense that was on last tick is off, and not because the one it was
        // about hatched, died, grew up or was freed. The body sensors still
        // hold last tick's senses until biochemistry.
        let was = |locus: usize| sprite.body.loci[locus] > 0.0;
        if (was(indices.egg_away) && !senses.egg_away && !seen.is_empty())
            || (was(indices.young_away) && !senses.young_away && young.is_some())
            || (was(indices.parent_away) && is_young && sees_parent)
            || (was(indices.partner_away) && sprite.partner().is_some_and(sees))
        {
            found.push(id);
        }
        company.family = senses;
        if !seen.is_empty() || !sprite.eggs.is_empty() {
            seen_eggs.push((id, seen));
        }
    }
    for id in found {
        let sprite = state.sprites.get_mut(id).expect("a sprite that looked");
        sprite.body.pulse(indices.found, None);
    }
    for (id, seen) in seen_eggs {
        let sprite = state.sprites.get_mut(id).expect("a sprite that looked");
        sprite
            .eggs
            .retain(|known| eggs.iter().any(|&(egg, ..)| egg == known.egg));
        for egg in seen {
            match sprite.eggs.iter_mut().find(|known| known.egg == egg.egg) {
                Some(known) => *known = egg,
                None => sprite.eggs.push(egg),
            }
        }
    }
}

impl Sprite {
    /// Forgets where it last saw its egg `egg` (M2 design §15.6): it went
    /// back there and didn't find it.
    pub(crate) fn forget_egg(&mut self, egg: EntityId) {
        self.eggs.retain(|known| known.egg != egg);
    }

    /// Forgets the tile a trip to `target` looked for it on: where the
    /// sprite heard it call (M2 design §15.1), or, for an egg of its own,
    /// where it last saw it (§15.6).
    pub(crate) fn forget_sought(&mut self, target: Target) {
        match target {
            Target::Sprite(caller) => self.forget_call(caller),
            Target::Object(egg) => self.forget_egg(egg),
            Target::Water(_) | Target::Cursor => {}
        }
    }
}
