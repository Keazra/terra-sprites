//! The family tree (M2 design §17): `k` opens it over the panels, on the
//! selected sprite or egg. Its grandparents and parents are drawn above it,
//! its young below, grouped by their other parent, and its brothers and
//! sisters at the foot, living or dead. The arrows or the pointer choose
//! one, which blinks, and `Enter` or a click walks the tree to it and
//! selects it. The lineage colours (`b`) mark the same sprites on the map.

use std::collections::BTreeMap;
use std::ops::Range;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use terra_sim::{DeadView, DeathCause, EggView, EntityId, LifeStage, Origin, SpriteView, World};

use crate::app::App;
use crate::inspector::{Hatching, hatching};
use crate::policy::{Panel, Subject};
use crate::text::{BOUND, cause_name, group_thousands, label, unbroken};

/// What the world knows of a sprite on the tree.
pub(crate) enum Known<'a> {
    Living(SpriteView<'a>),
    Egg(EggView<'a>),
    Dead(DeadView<'a>),
    /// One the world no longer remembers, dead long ago (M2 design §17.1).
    Forgotten,
}

impl<'a> Known<'a> {
    pub(crate) fn of(world: &'a World, id: EntityId) -> Known<'a> {
        if let Some(sprite) = world.sprite(id) {
            Known::Living(sprite)
        } else if let Some(egg) = world.egg(id) {
            Known::Egg(egg)
        } else if let Some(dead) = world.dead(id) {
            Known::Dead(dead)
        } else {
            Known::Forgotten
        }
    }

    /// Its parents, as far as the world knows them.
    fn parents(&self) -> &'a [EntityId] {
        match self {
            Known::Living(sprite) => sprite.parents(),
            Known::Egg(egg) => egg.parents(),
            Known::Dead(dead) => dead.parents(),
            Known::Forgotten => &[],
        }
    }

    /// The name the player gave it, as the world remembers it.
    fn name(&self) -> Option<&'a str> {
        match self {
            Known::Living(sprite) => sprite.name(),
            Known::Egg(egg) => egg.name(),
            Known::Dead(dead) => dead.name(),
            Known::Forgotten => None,
        }
    }

    /// Where it came from, if it has no parents: one the world began with,
    /// or one the Cursor placed. An egg always has parents, and the world
    /// doesn't know a forgotten sprite's.
    fn origin(&self) -> Option<&'static str> {
        match self {
            Known::Living(sprite) => origin_text(sprite.origin()),
            Known::Dead(dead) => origin_text(dead.origin()),
            Known::Egg(_) | Known::Forgotten => None,
        }
    }

    /// Whether it's dead, remembered or not.
    fn gone(&self) -> bool {
        matches!(self, Known::Dead(_) | Known::Forgotten)
    }
}

/// Where a sprite with no parents came from, as the screen says it (M2
/// design §17.2), or nothing for one that hatched.
pub(crate) fn origin_text(origin: Origin) -> Option<&'static str> {
    match origin {
        Origin::First => Some("one of the first sprites"),
        Origin::Placed => Some("placed by the Cursor"),
        Origin::Hatched => None,
    }
}

/// How a death reads: `died of old age`, `died young of dehydration`,
/// `died hurt by thornbush`. Young is said only of a sprite that died
/// before it grew up (M2 design §17.2).
fn death_text(dead: &DeadView, world: &World) -> String {
    let young = dead.life_stage() == LifeStage::Young;
    let cause = cause_name(dead.cause(), world.data());
    match (dead.cause(), young) {
        (DeathCause::HurtBy(_), true) => format!("died young, {cause}"),
        (DeathCause::HurtBy(_), false) => format!("died {cause}"),
        (_, true) => format!("died young of {cause}"),
        (_, false) => format!("died of {cause}"),
    }
}

/// When an egg hatches, as the tree says it.
fn hatching_text(egg: &EggView, world: &World) -> String {
    match hatching(egg, world) {
        Hatching::In(ticks) => format!("egg · hatches in {}", group_thousands(ticks)),
        Hatching::OncePutDown => "egg · hatches once it's put down".to_string(),
        Hatching::OnceClear => "egg · hatches once nothing stands on it".to_string(),
    }
}

/// Whether the policy lets the tree show sprite `id` (M1 design §6.4).
pub(crate) fn shown(app: &App, id: EntityId) -> bool {
    app.can_view(Panel::FamilyTree, Subject::Sprite(id))
}

/// How the tree names sprite `id`, as the screen does, or nothing if the
/// policy hides it. A sprite named before a save that has since died takes
/// its name from the world's record of it.
pub(crate) fn name_of(app: &App, world: &World, id: EntityId) -> String {
    if !shown(app, id) {
        return String::new();
    }
    label(id, app.names().get(id).or(Known::of(world, id).name()))
}

/// Where the tree draws a sprite: in which band, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Band {
    Grandparents,
    Parents,
    Centre,
    Young,
    Siblings,
}

/// A sprite on the tree: how it's named, what's said of it, in parts the
/// tree joins with ` · `, and whether it's dead, so drawn dark gray. One the
/// policy hides says nothing, but keeps its place, and can't be chosen (M2
/// design §17.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Card {
    pub(crate) id: EntityId,
    pub(crate) label: String,
    pub(crate) parts: Vec<String>,
    pub(crate) gone: bool,
    pub(crate) hidden: bool,
}

impl Card {
    /// The card of a sprite of the middle one's lineage (M2 design §17.2):
    /// living, its life stage and how many young it has; an egg, when it
    /// hatches; dead, how it died; forgotten, that it lived long ago. Then
    /// `more`, if any.
    fn of(app: &App, world: &World, id: EntityId, more: Option<String>) -> Card {
        let known = Known::of(world, id);
        if !shown(app, id) {
            return Card::hidden(id);
        }
        let mut parts = match &known {
            Known::Living(sprite) => {
                let mut parts = vec![sprite.life_stage().name().to_string()];
                let young = world.young(id).len();
                if young > 0 {
                    parts.push(format!("{young} young"));
                }
                parts
            }
            Known::Egg(egg) => vec![hatching_text(egg, world)],
            Known::Dead(dead) => vec![death_text(dead, world)],
            Known::Forgotten => vec!["lived long ago".to_string()],
        };
        parts.extend(more);
        Card {
            id,
            label: name_of(app, world, id),
            parts,
            gone: known.gone(),
            hidden: false,
        }
    }

    /// The card of the sprite in the middle: its life stage, sex, age and
    /// generation; an egg's, when it hatches; a dead one's, how and at what
    /// age it died.
    fn centre(app: &App, world: &World, id: EntityId) -> Card {
        let known = Known::of(world, id);
        if !shown(app, id) {
            return Card::hidden(id);
        }
        let text = match &known {
            Known::Living(sprite) => format!(
                "{} · {} · age {} · generation {}",
                sprite.life_stage().name(),
                sprite.sex().name(),
                group_thousands(sprite.age()),
                sprite.generation()
            ),
            Known::Egg(egg) => format!(
                "{} · generation {}",
                hatching_text(egg, world),
                egg.generation()
            ),
            Known::Dead(dead) => format!(
                "{} at age {} · generation {}",
                death_text(dead, world),
                group_thousands(dead.age()),
                dead.generation()
            ),
            Known::Forgotten => "lived long ago".to_string(),
        };
        Card {
            id,
            label: name_of(app, world, id),
            parts: vec![text],
            gone: known.gone(),
            hidden: false,
        }
    }

    /// A card the policy hides: blank, in its place.
    fn hidden(id: EntityId) -> Card {
        Card {
            id,
            label: String::new(),
            parts: Vec::new(),
            gone: false,
            hidden: true,
        }
    }
}

/// A parent and what's above it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Branch {
    pub(crate) parent: Card,
    pub(crate) above: Above,
}

/// What the tree shows above a parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Above {
    /// Its own parents.
    Grandparents(Vec<Card>),
    /// Where it came from, having no parents.
    Origin(&'static str),
    /// Nothing: the world no longer remembers it, or the policy hides
    /// where it came from.
    Nothing,
}

/// A sprite's young with one other parent, or laid alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Brood {
    /// `with Sprite #27476`, `partner Sprite #27476` for its partner's (M2
    /// design §17.2), or `alone`.
    pub(crate) heading: String,
    pub(crate) young: Vec<Card>,
}

/// A sprite's family tree, as the screen says it (M2 design §17.2): the
/// sprite in the middle, its parents and theirs above it, its young below,
/// and its brothers and sisters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamilyTree {
    pub(crate) centre: Card,
    pub(crate) parents: Vec<Branch>,
    /// Where it came from, in its parents' place, if it has none.
    pub(crate) origin: Option<&'static str>,
    pub(crate) broods: Vec<Brood>,
    pub(crate) siblings: Vec<Card>,
}

impl FamilyTree {
    /// The family tree of the sprite or egg `id`, living or dead.
    pub(crate) fn of(app: &App, world: &World, id: EntityId) -> FamilyTree {
        let known = Known::of(world, id);
        let parents = known.parents();
        let branches = parents
            .iter()
            .map(|&parent| {
                let theirs = Known::of(world, parent);
                // Where a parent came from is about it, so the policy may
                // hide it.
                let origin = theirs.origin().filter(|_| shown(app, parent));
                let above = match (theirs.parents(), origin) {
                    ([], Some(origin)) => Above::Origin(origin),
                    ([], None) => Above::Nothing,
                    (grandparents, _) => Above::Grandparents(
                        grandparents
                            .iter()
                            .map(|&id| Card::of(app, world, id, None))
                            .collect(),
                    ),
                };
                Branch {
                    parent: Card::of(app, world, parent, None),
                    above,
                }
            })
            .collect();
        // The young by their other parents, in the order they came, those
        // with its partner, if it has one, under the partner's name (M2
        // design §15.4).
        let partner = match &known {
            Known::Living(sprite) => sprite.partner().map(|(partner, _)| partner),
            _ => None,
        };
        let mut broods: Vec<(Vec<EntityId>, Brood)> = Vec::new();
        for young in world.young(id) {
            let others: Vec<EntityId> = Known::of(world, young)
                .parents()
                .iter()
                .copied()
                .filter(|&parent| parent != id)
                .collect();
            let card = Card::of(app, world, young, None);
            match broods.iter_mut().find(|(with, _)| *with == others) {
                Some((_, brood)) => brood.young.push(card),
                None => {
                    let heading = match others.as_slice() {
                        [] => "alone".to_string(),
                        others => {
                            let names: Vec<String> = others
                                .iter()
                                .map(|&other| name_of(app, world, other))
                                .collect();
                            // A parent the policy hides goes unnamed (M2
                            // design §17.6).
                            let with = if partner.is_some_and(|partner| others == [partner]) {
                                "partner"
                            } else {
                                "with"
                            };
                            if names.iter().any(String::is_empty) {
                                String::new()
                            } else {
                                format!("{with} {}", names.join(" and "))
                            }
                        }
                    };
                    let young = vec![card];
                    broods.push((others, Brood { heading, young }));
                }
            }
        }
        // One with other parents than this one's says which it shares.
        let siblings = siblings(world, id)
            .into_iter()
            .map(|sibling| {
                let theirs = Known::of(world, sibling).parents();
                let alike =
                    theirs.len() == parents.len() && theirs.iter().all(|p| parents.contains(p));
                let shared: Vec<String> = theirs
                    .iter()
                    .filter(|p| parents.contains(p))
                    .map(|&p| unbroken(&name_of(app, world, p)))
                    .collect();
                let named = shared.iter().all(|name| !name.is_empty());
                let through =
                    (!alike && named).then(|| format!("through {}", shared.join(" and ")));
                Card::of(app, world, sibling, through)
            })
            .collect();
        FamilyTree {
            centre: Card::centre(app, world, id),
            parents: branches,
            origin: known.origin(),
            broods: broods.into_iter().map(|(_, brood)| brood).collect(),
            siblings,
        }
    }

    /// The young, in the order the tree draws them.
    fn young(&self) -> impl Iterator<Item = &Card> {
        self.broods.iter().flat_map(|brood| &brood.young)
    }
}

/// How far along the rows that scroll sideways are: the first of the young,
/// and of the brothers and sisters, shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Scroll {
    pub(crate) young: usize,
    pub(crate) siblings: usize,
}

/// The columns a young sprite, or a brother or sister, takes in its row.
const COLUMN: usize = 24;

/// The rows a card's text may take, under its label.
const TEXT_ROWS: usize = 2;

// The tree's rows, from the top of the box's inside: each band's labels,
// with its cards' text under them, and the lines joining the bands.
const GRANDPARENTS: usize = 1;
const GRANDPARENTS_JOIN: usize = GRANDPARENTS + 1 + TEXT_ROWS;
const PARENTS: usize = GRANDPARENTS_JOIN + 1;
const PARENTS_JOIN: usize = PARENTS + 1 + TEXT_ROWS;
const CENTRE: usize = PARENTS_JOIN + 1;
const DOWN: usize = CENTRE + 2;
const BROODS_JOIN: usize = DOWN + 1;
const BROODS: usize = BROODS_JOIN + 1;
const BROOD_JOIN: usize = BROODS + 1;
const YOUNG: usize = BROOD_JOIN + 1;
const SIBLINGS_HEADING: usize = YOUNG + 1 + TEXT_ROWS + 1;
const SIBLINGS: usize = SIBLINGS_HEADING + 1;

/// A card on the tree: whose it is, and where. A sprite can be on the tree
/// twice, as a parent and a grandparent when its young had young together,
/// so the arrows go from card to card, not from sprite to sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spot {
    pub(crate) id: EntityId,
    band: Band,
    /// Its order in its band, from the left.
    index: usize,
}

/// Where a sprite's card is on the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    spot: Spot,
    /// The column its card is centred on: in a row that scrolls, maybe off
    /// the tree's edge.
    x: i32,
    /// How wide its card is.
    width: usize,
    /// The row of its label; its text is under it.
    row: usize,
    /// The columns its label takes.
    label: Range<i32>,
    /// Whether it's in view, in a row that scrolls.
    shown: bool,
    /// Whether the policy hides it: a blank card, which the arrows and the
    /// pointer pass by (M2 design §17.6).
    hidden: bool,
}

impl Place {
    /// Whether the cell at column `x` and row `y` of the tree is on its card.
    fn contains(&self, x: i32, y: usize) -> bool {
        let left = self.x - (self.width / 2) as i32;
        self.shown
            && (self.row..=self.row + TEXT_ROWS).contains(&y)
            && (left..left + self.width as i32).contains(&x)
    }
}

/// A way the arrows walk the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Left,
    Right,
    Up,
    Down,
}

/// The tree as drawn: its lines, where each card is on it, and the card
/// chosen.
pub(crate) struct Drawn {
    pub(crate) lines: Vec<Line<'static>>,
    places: Vec<Place>,
    /// How many of the young, or of the brothers and sisters, fit across.
    across: usize,
    /// The card the arrows are on.
    pub(crate) chosen: Spot,
}

impl Drawn {
    /// The card the arrows reach from `from` by `step`: the one beside it in
    /// its band, or the nearest in the next band up or down that has any.
    /// A card the policy hides is passed by (M2 design §17.6).
    pub(crate) fn step(&self, from: Spot, step: Step) -> Option<Spot> {
        let here = self.place(from)?;
        let choosable = self.places.iter().filter(|place| !place.hidden);
        let band = |band: Band| {
            choosable
                .clone()
                .filter(move |place| place.spot.band == band)
        };
        let (at, index) = (here.spot.band, here.spot.index);
        let next = match step {
            Step::Left => band(at)
                .filter(|place| place.spot.index < index)
                .max_by_key(|place| place.spot.index),
            Step::Right => band(at)
                .filter(|place| place.spot.index > index)
                .min_by_key(|place| place.spot.index),
            Step::Up | Step::Down => {
                let bands = choosable
                    .clone()
                    .map(|place| place.spot.band)
                    .filter(|&b| if step == Step::Up { b < at } else { b > at });
                let to = if step == Step::Up {
                    bands.max()
                } else {
                    bands.min()
                }?;
                band(to).min_by_key(|place| ((place.x - here.x).abs(), place.spot.index))
            }
        };
        next.map(|place| place.spot)
    }

    /// The card at column `x` and row `y` of the tree, unless the policy
    /// hides it (M2 design §17.6).
    pub(crate) fn at(&self, x: i32, y: usize) -> Option<Spot> {
        self.places
            .iter()
            .find(|place| !place.hidden && place.contains(x, y))
            .map(|place| place.spot)
    }

    /// Scrolls the row `spot` is in, if it's one that scrolls, the least
    /// that brings it into view.
    pub(crate) fn keep_in_view(&self, scroll: &mut Scroll, spot: Spot) {
        let first = match spot.band {
            Band::Young => &mut scroll.young,
            Band::Siblings => &mut scroll.siblings,
            _ => return,
        };
        if spot.index < *first {
            *first = spot.index;
        } else if spot.index >= *first + self.across {
            *first = spot.index + 1 - self.across;
        }
    }

    fn place(&self, spot: Spot) -> Option<&Place> {
        self.places.iter().find(|place| place.spot == spot)
    }
}

/// The card `spot` was, on a tree whose cards are at `places`, which may
/// have changed since: the same card, if it's there; or else, as young are
/// born and die, the same sprite's card in the same band, or anywhere; or
/// else the sprite in the middle's. Never one the policy hides now (M2
/// design §17.6).
fn find_again(places: &[Place], spot: Option<Spot>) -> Spot {
    let of = |id: EntityId| {
        places
            .iter()
            .filter(move |place| place.spot.id == id && !place.hidden)
    };
    spot.and_then(|spot| {
        of(spot.id)
            .find(|place| place.spot == spot)
            .or_else(|| of(spot.id).find(|place| place.spot.band == spot.band))
            .or_else(|| of(spot.id).next())
    })
    .or_else(|| places.iter().find(|place| place.spot.band == Band::Centre))
    .map(|place| place.spot)
    .expect("a tree has a sprite in the middle")
}

/// Draws `family_tree` `width` columns wide, its rows that scroll as far along
/// as `scroll` says, with the card `chosen`, or the one in the middle,
/// blinking: its label in reverse video while `blink` is on (M2 design
/// §17.3).
pub(crate) fn draw(
    family_tree: &FamilyTree,
    width: usize,
    scroll: Scroll,
    chosen: Option<Spot>,
    blink: bool,
) -> Drawn {
    let mut tree = Tree {
        canvas: Canvas::new(width, SIBLINGS + 1 + TEXT_ROWS),
        places: Vec::new(),
    };
    // Four grandparents side by side, each parent centred over its own.
    let quarter = width / 4;
    let slot = |at: usize| ((width - 4 * quarter) / 2 + at * quarter + quarter / 2) as i32;
    let centre = (slot(1) + slot(2)) / 2;
    let (parents, slots): (Vec<i32>, &[[usize; 2]]) = match family_tree.parents.len() {
        1 => (vec![centre], &[[1, 2]]),
        _ => (
            vec![(slot(0) + slot(1)) / 2, (slot(2) + slot(3)) / 2],
            &[[0, 1], [2, 3]],
        ),
    };
    let mut grandparents = 0;
    for ((branch, &x), slots) in family_tree.parents.iter().zip(&parents).zip(slots) {
        match &branch.above {
            Above::Grandparents(cards) => {
                let xs: Vec<i32> = match cards.len() {
                    1 => vec![x],
                    _ => slots.iter().map(|&at| slot(at)).collect(),
                };
                for (card, &x) in cards.iter().zip(&xs) {
                    tree.card(
                        card,
                        Band::Grandparents,
                        grandparents,
                        x,
                        quarter,
                        GRANDPARENTS,
                    );
                    grandparents += 1;
                }
                tree.canvas.join(GRANDPARENTS_JOIN, &xs, &[x]);
            }
            Above::Origin(text) => {
                tree.canvas
                    .centred(x, GRANDPARENTS, text, 2 * quarter - 1, Style::default());
            }
            Above::Nothing => {}
        }
    }
    for (index, (branch, &x)) in family_tree.parents.iter().zip(&parents).enumerate() {
        tree.card(
            &branch.parent,
            Band::Parents,
            index,
            x,
            2 * quarter,
            PARENTS,
        );
    }
    if family_tree.parents.is_empty() {
        if let Some(origin) = family_tree.origin {
            tree.canvas
                .centred(centre, PARENTS, origin, width, Style::default());
        }
    } else {
        tree.canvas.join(PARENTS_JOIN, &parents, &[centre]);
    }
    tree.card(
        &family_tree.centre,
        Band::Centre,
        0,
        centre,
        width + 1,
        CENTRE,
    );
    // The young, under a heading for each other parent.
    let young: Vec<&Card> = family_tree.young().collect();
    let (xs, shown) = strip_columns(young.len(), width, scroll.young);
    if !young.is_empty() {
        tree.canvas.put(centre, DOWN, "│", Style::default());
        let mut headings = Vec::new();
        let mut from = 0;
        for brood in &family_tree.broods {
            let all = from..from + brood.young.len();
            from = all.end;
            let seen: Vec<usize> = all.clone().filter(|at| shown.contains(at)).collect();
            let (Some(&first), Some(&last)) = (seen.first(), seen.last()) else {
                headings.push((xs[all.start] + xs[all.end - 1]) / 2);
                continue;
            };
            let x = (xs[first] + xs[last]) / 2;
            headings.push(x);
            let room = seen.len() * COLUMN - 1;
            tree.canvas
                .centred(x, BROODS, &brood.heading, room, Style::default());
            let under: Vec<i32> = seen.iter().map(|&at| xs[at]).collect();
            tree.canvas.join(BROOD_JOIN, &[x], &under);
        }
        tree.canvas.join(BROODS_JOIN, &[centre], &headings);
        tree.strip(&young, Band::Young, &xs, &shown, YOUNG);
    }
    if !family_tree.siblings.is_empty() {
        let siblings: Vec<&Card> = family_tree.siblings.iter().collect();
        let (xs, shown) = strip_columns(siblings.len(), width, scroll.siblings);
        tree.canvas.put(
            1,
            SIBLINGS_HEADING,
            "BROTHERS AND SISTERS",
            Style::default(),
        );
        tree.strip(&siblings, Band::Siblings, &xs, &shown, SIBLINGS);
    }
    let chosen = find_again(&tree.places, chosen);
    if blink && let Some(place) = tree.places.iter().find(|place| place.spot == chosen) {
        tree.canvas
            .mark(place.row, place.label.clone(), Modifier::REVERSED);
    }
    Drawn {
        lines: tree.canvas.lines(),
        places: tree.places,
        across: across(width),
        chosen,
    }
}

/// How many of a row that scrolls fit across `width` columns.
fn across(width: usize) -> usize {
    (width / COLUMN).max(1)
}

/// Where each of `count` cards in a row that scrolls is centred, and which
/// of them are shown: all of them, centred, if they fit; otherwise as many
/// as fit, from the one at `first`, or as near it as leaves none
/// unfilled.
fn strip_columns(count: usize, width: usize, first: usize) -> (Vec<i32>, Range<usize>) {
    let across = across(width);
    let shown = count.min(across);
    let first = first.min(count - shown);
    let left = (width.saturating_sub(shown * COLUMN) / 2) as i32;
    let xs = (0..count)
        .map(|at| left + (at as i32 - first as i32) * COLUMN as i32 + COLUMN as i32 / 2)
        .collect();
    (xs, first..first + shown)
}

/// A tree being drawn.
struct Tree {
    canvas: Canvas,
    places: Vec<Place>,
}

impl Tree {
    /// Draws `card` centred on column `x`, its label on row `row` and its
    /// text under it, within `width` columns, and notes its place as the
    /// `index`th of `band`.
    fn card(&mut self, card: &Card, band: Band, index: usize, x: i32, width: usize, row: usize) {
        let room = width - 1;
        let style = if card.gone {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };
        let label = self.canvas.centred(x, row, &card.label, room, style);
        let rows = if band == Band::Centre { 1 } else { TEXT_ROWS };
        for (below, line) in wrap(&card.parts, room, rows).iter().enumerate() {
            self.canvas.centred(x, row + 1 + below, line, room, style);
        }
        self.places.push(Place {
            spot: Spot {
                id: card.id,
                band,
                index,
            },
            x,
            width,
            row,
            label,
            shown: true,
            hidden: card.hidden,
        });
    }

    /// Draws a row that scrolls: the cards shown at `xs`, with `◄` or `►`
    /// at an edge past which there are more.
    fn strip(&mut self, cards: &[&Card], band: Band, xs: &[i32], shown: &Range<usize>, row: usize) {
        for (index, (card, &x)) in cards.iter().zip(xs).enumerate() {
            if shown.contains(&index) {
                self.card(card, band, index, x, COLUMN, row);
            } else {
                self.places.push(Place {
                    spot: Spot {
                        id: card.id,
                        band,
                        index,
                    },
                    x,
                    width: COLUMN,
                    row,
                    label: x..x,
                    shown: false,
                    hidden: card.hidden,
                });
            }
        }
        if shown.start > 0 {
            self.canvas.put(0, row, "◄", Style::default());
        }
        if shown.end < cards.len() {
            let right = self.canvas.width as i32 - 1;
            self.canvas.put(right, row, "►", Style::default());
        }
    }
}

/// `parts` on lines of at most `width` columns, at most `rows` of them: as
/// many parts on a line as fit, joined by ` · `, and a part too long for a
/// line of its own wrapped at its spaces. A word too long for a line is
/// cut.
fn wrap(parts: &[String], width: usize, rows: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for part in parts {
        let joined = if line.is_empty() {
            part.clone()
        } else {
            format!("{line} · {part}")
        };
        if joined.chars().count() <= width {
            line = joined;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        for word in part.split(' ') {
            let used = line.chars().count();
            if used > 0 && used + 1 + word.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
        .into_iter()
        .take(rows)
        .map(|line| line.replace(BOUND, " ").chars().take(width).collect())
        .collect()
}

/// Cells to draw text and lines on, a row of characters at a time, each
/// with its style.
struct Canvas {
    width: usize,
    rows: Vec<Vec<(char, Style)>>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Canvas {
        Canvas {
            width,
            rows: vec![vec![(' ', Style::default()); width]; height],
        }
    }

    /// Draws `text` from column `x` of row `y`, leaving off what's past
    /// either edge.
    fn put(&mut self, x: i32, y: usize, text: &str, style: Style) {
        let Some(row) = self.rows.get_mut(y) else {
            return;
        };
        for (at, c) in (x..).zip(text.chars()) {
            if let Some(cell) = usize::try_from(at).ok().and_then(|at| row.get_mut(at)) {
                *cell = (c, style);
            }
        }
    }

    /// Draws `text`, cut to `room` columns, centred on column `x` of row
    /// `y`, returning the columns it takes.
    fn centred(&mut self, x: i32, y: usize, text: &str, room: usize, style: Style) -> Range<i32> {
        let text: String = text.chars().take(room).collect();
        let length = text.chars().count() as i32;
        let from = x - length / 2;
        self.put(from, y, &text, style);
        from..from + length
    }

    /// Adds `modifier` to the cells of row `y` in `columns`.
    fn mark(&mut self, y: usize, columns: Range<i32>, modifier: Modifier) {
        let Some(row) = self.rows.get_mut(y) else {
            return;
        };
        for at in columns {
            if let Some((_, style)) = usize::try_from(at).ok().and_then(|at| row.get_mut(at)) {
                *style = style.add_modifier(modifier);
            }
        }
    }

    /// Draws the line on row `y` joining the cards centred on `ups`, above
    /// it, to those on `downs`, below: `└──┬──┘`, `┌──┼──┐` and the like,
    /// running past the edge where there's more beyond it.
    fn join(&mut self, y: usize, ups: &[i32], downs: &[i32]) {
        let ends = ups.iter().chain(downs);
        let (Some(&from), Some(&to)) = (ends.clone().min(), ends.max()) else {
            return;
        };
        let last = self.width as i32 - 1;
        for x in from.max(0)..=to.min(last) {
            let joint = joint(ups.contains(&x), downs.contains(&x), x > from, x < to);
            self.put(x, y, &joint.to_string(), Style::default());
        }
    }

    /// The canvas as lines, a span for each run of one style.
    fn lines(self) -> Vec<Line<'static>> {
        self.rows
            .into_iter()
            .map(|row| {
                let mut spans: Vec<Span<'static>> = Vec::new();
                let mut run = String::new();
                let mut style = Style::default();
                for (c, cell) in row {
                    if cell != style && !run.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut run), style));
                    }
                    style = cell;
                    run.push(c);
                }
                spans.push(Span::styled(run, style));
                Line::from(spans)
            })
            .collect()
    }
}

/// The box-drawing character that joins lines going up, down, left and
/// right from a cell, as each says.
fn joint(up: bool, down: bool, left: bool, right: bool) -> char {
    match (up, down, left, right) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, false, true, true) => '┴',
        (false, true, true, true) => '┬',
        (true, false, true, false) => '┘',
        (true, false, false, true) => '└',
        (false, true, true, false) => '┐',
        (false, true, false, true) => '┌',
        (true, _, false, false) | (false, true, false, false) => '│',
        (false, false, _, _) => '─',
    }
}

/// The brothers and sisters of the sprite `id`: the young of either of its
/// parents, in ascending ID order (M2 design §17.2).
fn siblings(world: &World, id: EntityId) -> Vec<EntityId> {
    let mut siblings: Vec<EntityId> = Known::of(world, id)
        .parents()
        .iter()
        .flat_map(|&parent| world.young(parent))
        .filter(|&sibling| sibling != id)
        .collect();
    siblings.sort();
    siblings.dedup();
    siblings
}

/// Where a sprite on the map stands in the selected one's lineage, for the
/// lineage colours (M2 design §17.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lineage {
    /// A parent or grandparent.
    Elder,
    /// A brother or sister.
    Sibling,
    /// One of its young, or theirs.
    Young,
}

/// The sprites of sprite `id`'s lineage the lineage colours mark (M2
/// design §17.4): its parents and grandparents, its brothers and sisters, and
/// its young and theirs, living or dead or eggs. One that's there twice
/// over, as in an inbred line, takes the first of these that it is.
pub(crate) fn lineage(world: &World, id: EntityId) -> BTreeMap<EntityId, Lineage> {
    let mut marked = BTreeMap::new();
    let parents = Known::of(world, id).parents();
    for &parent in parents {
        marked.entry(parent).or_insert(Lineage::Elder);
        for &grandparent in Known::of(world, parent).parents() {
            marked.entry(grandparent).or_insert(Lineage::Elder);
        }
    }
    for young in world.young(id) {
        marked.entry(young).or_insert(Lineage::Young);
        for theirs in world.young(young) {
            marked.entry(theirs).or_insert(Lineage::Young);
        }
    }
    for sibling in siblings(world, id) {
        marked.entry(sibling).or_insert(Lineage::Sibling);
    }
    marked.remove(&id);
    marked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| part.to_string()).collect()
    }

    #[test]
    fn a_card_s_text_has_as_many_parts_on_a_line_as_fit() {
        assert_eq!(
            wrap(&parts(&["adult", "3 young"]), 23, 2),
            ["adult · 3 young"]
        );
        let through = format!("through {}", unbroken("Sprite #3"));
        assert_eq!(
            wrap(&parts(&["adult", "3 young", &through]), 23, 2),
            ["adult · 3 young", "through Sprite #3"]
        );
    }

    #[test]
    fn a_long_part_breaks_at_its_spaces_but_never_inside_a_name() {
        let through = format!(
            "through {} and {}",
            unbroken("Old Bramble #1"),
            unbroken("Pip #2")
        );
        assert_eq!(
            wrap(&[through], 23, 2),
            ["through Old Bramble #1", "and Pip #2"]
        );
    }

    #[test]
    fn a_card_s_text_stops_at_its_rows_and_a_word_too_long_is_cut() {
        assert_eq!(
            wrap(&parts(&["died young of dehydration"]), 12, 2),
            ["died young", "of"]
        );
        assert_eq!(
            wrap(&parts(&["Supercalifragilistic"]), 10, 2),
            ["Supercalif"]
        );
    }

    /// Row `y` of `canvas`, as text.
    fn row(canvas: &Canvas, y: usize) -> String {
        canvas.rows[y].iter().map(|&(c, _)| c).collect()
    }

    #[test]
    fn the_lines_join_cards_above_to_cards_below() {
        let mut canvas = Canvas::new(7, 5);
        canvas.join(0, &[1, 5], &[3]);
        canvas.join(1, &[3], &[1, 3, 5]);
        canvas.join(2, &[3], &[1, 5]);
        canvas.join(3, &[3], &[3]);
        canvas.join(4, &[1], &[3, 9]);
        assert_eq!(row(&canvas, 0), " └─┬─┘ ");
        assert_eq!(row(&canvas, 1), " ┌─┼─┐ ");
        assert_eq!(row(&canvas, 2), " ┌─┴─┐ ");
        assert_eq!(row(&canvas, 3), "   │   ");
        assert_eq!(row(&canvas, 4), " └─┬───", "on past the edge");
    }

    #[test]
    fn a_row_that_scrolls_shows_as_many_as_fit_from_the_first_asked() {
        // Two fit in 98 columns: centred, four columns of 24 across.
        assert_eq!(strip_columns(2, 98, 0), (vec![37, 61], 0..2));
        // Seven don't: four, from the one asked, but none left unfilled.
        let (xs, shown) = strip_columns(7, 98, 2);
        assert_eq!((xs[2], shown), (13, 2..6));
        let (xs, shown) = strip_columns(7, 98, 5);
        assert_eq!((xs[3], xs[6], shown), (13, 85, 3..7));
        // Narrower than a column, one.
        assert_eq!(strip_columns(3, 10, 1).1, 1..2);
    }

    fn spot(id: u64, band: Band, index: usize) -> Spot {
        Spot {
            id: EntityId(id),
            band,
            index,
        }
    }

    fn place(spot: Spot) -> Place {
        Place {
            spot,
            x: 0,
            width: 1,
            row: 0,
            label: 0..0,
            shown: true,
            hidden: false,
        }
    }

    #[test]
    fn the_card_chosen_is_found_again_as_the_tree_changes() {
        let centre = spot(9, Band::Centre, 0);
        let places = [
            place(centre),
            place(spot(4, Band::Parents, 0)),
            place(spot(4, Band::Siblings, 0)),
            place(spot(5, Band::Young, 1)),
        ];
        let find_again = |spot| find_again(&places, spot);
        assert_eq!(find_again(None), centre);
        let twice = spot(4, Band::Siblings, 0);
        assert_eq!(find_again(Some(twice)), twice, "the card itself");
        assert_eq!(
            find_again(Some(spot(5, Band::Young, 0))),
            spot(5, Band::Young, 1),
            "moved along, as an egg joined a brood before it"
        );
        assert_eq!(
            find_again(Some(spot(5, Band::Siblings, 2))),
            spot(5, Band::Young, 1),
            "elsewhere"
        );
        assert_eq!(find_again(Some(spot(6, Band::Young, 0))), centre, "gone");
    }

    #[test]
    fn a_card_the_policy_hides_now_is_never_found_again() {
        // M2 design §17.6: one chosen before the policy hid it.
        let centre = spot(9, Band::Centre, 0);
        let hidden = Place {
            hidden: true,
            ..place(spot(5, Band::Young, 0))
        };
        assert_eq!(
            find_again(&[place(centre), hidden], Some(spot(5, Band::Young, 0))),
            centre
        );
    }
}
