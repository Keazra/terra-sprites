//! The Brain tab (design §5.9, §6.1): what the sprite attended to and
//! decided, and its memory, grouped under each thing it has learned about
//! (v40).

use std::collections::{BTreeMap, BTreeSet};

use ratatui::layout::{Margin, Position};
use terra_sim::{Dir, EntityId, Explanation, Learned, Part, Pos, SpriteView, Thing, Verb, World};

use super::actions::verb_name;
use super::{CURSOR, WIDTH, counted, first_shown, shown};
use crate::app::{App, Tab};
use crate::text::{BOUND, Words, display_name, level, signed_level, unbroken};

/// How many concepts the Brain tab lists under the decision.
const CONCEPTS_SHOWN: usize = 5;

/// How many lessons the Brain tab shows under a thing before "and N more"
/// (design v40 §6.1). What it thinks of sprites in general is not capped.
const LESSONS_SHOWN: usize = 3;

/// The sprite category's name. Every pack has it (design v19 §3.5.5).
const SPRITE_KIND: &str = "sprite";

/// A memory block the player can open on the Brain tab (design v40 §6.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MemoryOpen {
    /// An object type, a category, or the Cursor.
    Thing(Thing),
    /// New things, which aren't a thing in the pack.
    NewThings,
    /// One remembered sprite, nested under Sprites.
    Sprite(EntityId),
}

/// The memory block under screen cell `at`, if that line opens or shuts one.
pub(crate) fn lesson_at(app: &App, world: &World, at: Position) -> Option<MemoryOpen> {
    if app.tab() != Tab::Brain {
        return None;
    }
    let area = app.inspector_bounds()?;
    let inner = area.inner(Margin::new(1, 1));
    if at.y < inner.y || at.y >= inner.bottom() || at.x < inner.x || at.x >= inner.right() {
        return None;
    }
    // The same rows the inspector draws, policy included, so a hidden tab
    // has nothing to open.
    let rows = shown(app, world);
    let row = usize::from(at.y - inner.y);
    let first = first_shown(app.tab_scroll(), rows.len(), usize::from(inner.height));
    rows.get(first + row).and_then(|(_, open)| open.clone())
}

/// The Brain tab (design §5.9, §6.1): what the sprite attended to and
/// decided at its latest step 5, or "Nothing decided yet", or while led
/// "Being led: it decides nothing"; then its memory, grouped under each
/// thing (v40), which it can have before it first decides. A line that
/// opens or shuts a block carries that block.
pub(super) fn brain_rows(
    sprite: &SpriteView,
    app: &App,
    world: &World,
) -> Vec<(String, Option<MemoryOpen>)> {
    let data = &app.words(world);
    // Led, it decides nothing, so its last decision would mislead (design
    // v23 §2.4). Sliding, likewise (design v25 §2.4).
    let mut lines: Vec<(String, Option<MemoryOpen>)> = match sprite.explain() {
        _ if sprite.slide().is_some() => vec![(" Shoved: it decides nothing".into(), None)],
        _ if sprite.lead_steps_left().is_some() => {
            vec![(" Being led: it decides nothing".into(), None)]
        }
        Some(explained) => explained_lines(&explained, data)
            .into_iter()
            .map(|text| (text, None))
            .collect(),
        None => vec![(" Nothing decided yet".into(), None)],
    };
    let remembered: Vec<Lesson> = sprite
        .memory()
        .into_iter()
        .filter(|memory| level(memory.amount.abs()) != ".00")
        .map(|memory| Lesson {
            learned: memory.learned,
            amount: memory.amount,
        })
        .collect();
    // How well it knows each sprite it remembers (M2 design §15.8).
    let known: BTreeMap<EntityId, f32> = sprite
        .known_sprites()
        .into_iter()
        .filter(|&(_, familiarity)| level(familiarity) != ".00")
        .collect();
    let mut gap = true;
    let blocks = memory_blocks(remembered, &known);
    if !blocks.is_empty() {
        lines.push((String::new(), None));
        gap = false;
        for block in blocks {
            push_block(&mut lines, block, data, app.lessons_open(), &known);
        }
    }
    // Its remembered places, best remembered first (M2 design §7.5): what,
    // how far and which way, and how well, as the number alone.
    let places: Vec<_> = sprite
        .remembered_places()
        .into_iter()
        .filter(|p| level(p.recall) != ".00")
        .collect();
    if !places.is_empty() {
        if gap {
            lines.push((String::new(), None));
        }
        lines.push((" PLACES".into(), None));
    }
    for place in places {
        let name = format!(
            "{} · {}",
            thing_name(&place.thing, data),
            how_far(sprite.pos(), place.at)
        );
        for row in scored("   ", &name, &level(place.recall)) {
            lines.push((row, None));
        }
    }
    lines
}

/// One learned value, already in the memory list's order: furthest from
/// nothing first, a tie as it was listed (design v40 §5.9).
struct Lesson {
    learned: Learned,
    amount: f32,
}

/// Where a lesson is drawn: under its own heading, or under Sprites.
enum Block {
    Headed {
        key: MemoryOpen,
        lessons: Vec<Lesson>,
    },
    Sprites {
        general: Vec<Lesson>,
        individuals: Vec<(EntityId, Vec<Lesson>)>,
    },
}

/// Groups lessons under each thing. `remembered` is already loudest first,
/// ties in listed order, so appending as they come keeps that order, and a
/// block is placed where its loudest lesson sits. No second sort. A sprite
/// it `known`s but has learned nothing about follows those it has, best
/// known first (M2 design §15.8).
fn memory_blocks(remembered: Vec<Lesson>, known: &BTreeMap<EntityId, f32>) -> Vec<Block> {
    let mut blocks = Vec::new();
    for lesson in remembered {
        match lesson_home(&lesson.learned) {
            Home::Headed(key) => match headed(&mut blocks, &key) {
                Some(lessons) => lessons.push(lesson),
                None => blocks.push(Block::Headed {
                    key,
                    lessons: vec![lesson],
                }),
            },
            Home::General => sprites(&mut blocks).0.push(lesson),
            Home::Sprite(id) => {
                let individuals = sprites(&mut blocks).1;
                match individuals.iter_mut().find(|(got, _)| *got == id) {
                    Some((_, lessons)) => lessons.push(lesson),
                    None => individuals.push((id, vec![lesson])),
                }
            }
        }
    }
    let mut by_sight: Vec<(EntityId, f32)> = known
        .iter()
        .map(|(&id, &familiarity)| (id, familiarity))
        .collect();
    by_sight.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (id, _) in by_sight {
        let listed = blocks.iter().any(|block| match block {
            Block::Sprites { individuals, .. } => individuals.iter().any(|(got, _)| *got == id),
            Block::Headed { .. } => false,
        });
        if !listed {
            sprites(&mut blocks).1.push((id, Vec::new()));
        }
    }
    blocks
}

/// The lessons under `key`, if that heading is already open.
fn headed<'a>(blocks: &'a mut [Block], key: &MemoryOpen) -> Option<&'a mut Vec<Lesson>> {
    blocks.iter_mut().find_map(|block| match block {
        Block::Headed { key: got, lessons } if got == key => Some(lessons),
        _ => None,
    })
}

/// Sprites in general, and each remembered sprite, creating that block
/// where the first of either is met.
fn sprites(blocks: &mut Vec<Block>) -> (&mut Vec<Lesson>, &mut Vec<(EntityId, Vec<Lesson>)>) {
    if !blocks
        .iter()
        .any(|block| matches!(block, Block::Sprites { .. }))
    {
        blocks.push(Block::Sprites {
            general: Vec::new(),
            individuals: Vec::new(),
        });
    }
    match blocks
        .iter_mut()
        .find(|block| matches!(block, Block::Sprites { .. }))
    {
        Some(Block::Sprites {
            general,
            individuals,
        }) => (general, individuals),
        _ => unreachable!("the Sprites block was just put there"),
    }
}

/// Where one lesson is listed.
enum Home {
    /// An object, a category, the Cursor, or new things.
    Headed(MemoryOpen),
    /// Sprites in general, or a habit about sprites as a kind.
    General,
    Sprite(EntityId),
}

fn lesson_home(learned: &Learned) -> Home {
    match learned {
        Learned::NewThings => Home::Headed(MemoryOpen::NewThings),
        Learned::Worth { thing, .. }
        | Learned::Bad { thing }
        | Learned::Fear { thing }
        | Learned::Habit { thing, .. } => match thing {
            Thing::Sprite(id) => Home::Sprite(*id),
            Thing::Category(name) | Thing::ObjectType(name) if name == SPRITE_KIND => Home::General,
            thing => Home::Headed(MemoryOpen::Thing(thing.clone())),
        },
    }
}

fn push_block(
    lines: &mut Vec<(String, Option<MemoryOpen>)>,
    block: Block,
    data: &Words,
    open: &BTreeSet<MemoryOpen>,
    known: &BTreeMap<EntityId, f32>,
) {
    match block {
        Block::Headed { key, lessons } => {
            let name = heading_name(&key, data);
            push_headed(lines, " ", &name, None, lessons, key, data, open);
        }
        Block::Sprites {
            general,
            individuals,
        } => {
            lines.push((" SPRITES".into(), None));
            for lesson in general {
                push_lesson(lines, &lesson, "   ", data);
            }
            for (id, lessons) in individuals {
                let key = MemoryOpen::Sprite(id);
                let name = sprite_heading(id, data);
                let familiarity = known.get(&id).copied();
                push_headed(lines, "   ", &name, familiarity, lessons, key, data, open);
            }
        }
    }
}

/// A heading's words: the thing in capitals, or `NEW THINGS`.
fn heading_name(key: &MemoryOpen, data: &Words) -> String {
    match key {
        MemoryOpen::Thing(thing) => things(thing, data).0.to_uppercase(),
        MemoryOpen::NewThings => "NEW THINGS".into(),
        MemoryOpen::Sprite(id) => sprite_heading(*id, data),
    }
}

/// A heading, then how well it knows a remembered sprite, if it does, and
/// its lessons: three of them, or all if `key` is open.
#[expect(clippy::too_many_arguments, reason = "a heading and all it holds")]
fn push_headed(
    lines: &mut Vec<(String, Option<MemoryOpen>)>,
    head: &str,
    name: &str,
    familiarity: Option<f32>,
    lessons: Vec<Lesson>,
    key: MemoryOpen,
    data: &Words,
    open: &BTreeSet<MemoryOpen>,
) {
    let folds = lessons.len() > LESSONS_SHOWN;
    let opened = open.contains(&key);
    let marker = folds.then_some(if opened { "▼" } else { "►" });
    let heading = match marker {
        Some(marker) => format!("{head}{marker} {name}"),
        None => format!("{head}{name}"),
    };
    let toggle = folds.then_some(key.clone());
    lines.push((heading, toggle));
    let shown = if folds && !opened {
        &lessons[..LESSONS_SHOWN]
    } else {
        &lessons[..]
    };
    // Lessons sit two columns in from the heading.
    let lesson_indent = format!("{head}  ");
    if let Some(familiarity) = familiarity {
        for row in scored(&lesson_indent, "known", &level(familiarity)) {
            lines.push((row, None));
        }
    }
    for lesson in shown {
        push_lesson(lines, lesson, &lesson_indent, data);
    }
    if folds && !opened {
        let rest = lessons.len() - LESSONS_SHOWN;
        lines.push((format!("{lesson_indent}and {rest} more"), Some(key)));
    }
}

fn push_lesson(
    lines: &mut Vec<(String, Option<MemoryOpen>)>,
    lesson: &Lesson,
    indent: &str,
    data: &Words,
) {
    let text = lesson_text(&lesson.learned, lesson.amount, data);
    for row in scored(indent, &text, &signed_level(lesson.amount)) {
        lines.push((row, None));
    }
}

/// A lesson without the thing's name: the heading already says it.
fn lesson_text(learned: &Learned, amount: f32, data: &Words) -> String {
    match learned {
        Learned::Worth {
            need: Some(need), ..
        } => format!("good for {}", display_name(need)),
        Learned::Worth { need: None, .. } => "good".into(),
        Learned::Bad { .. } => "bad".into(),
        Learned::Fear { .. } => "frightening".into(),
        Learned::Habit { thing, verb } => {
            let pronoun = if things(thing, data).1 == "are" {
                "them"
            } else {
                "it"
            };
            format!("{} {pronoun}", doing(*verb))
        }
        Learned::NewThings => {
            if amount < 0.0 {
                "bad".into()
            } else {
                "good".into()
            }
        }
    }
}

/// A remembered sprite's heading: the name the player gave it, or its
/// number. The Sprites heading already says what it is.
fn sprite_heading(id: EntityId, data: &Words) -> String {
    match data.names.get(id) {
        Some(name) => name.to_string(),
        None => format!("#{}", id.0),
    }
}

/// How far `to` is from `from` and which way, in tiles, the game's own
/// measure: `34 tiles NE`, or `here`.
fn how_far(from: Pos, to: Pos) -> String {
    let (dx, dy) = (
        i32::from(to.x) - i32::from(from.x),
        i32::from(to.y) - i32::from(from.y),
    );
    match Dir::nearest(dx, dy) {
        Some(toward) => {
            let tiles = dx.unsigned_abs().max(dy.unsigned_abs());
            format!("{} {toward:?}", counted(tiles, "tile"))
        }
        None => "here".into(),
    }
}

/// The Brain tab's attention and decision: each category attention could go
/// to, with its score, the attended one marked; then the verb chosen or
/// kept doing, with its score, and the concepts adding most to it, largest
/// first, or "none" for a sprite standing with nothing to do (M2 design
/// §19.2).
fn explained_lines(explained: &Explanation, data: &Words) -> Vec<String> {
    let mut lines = vec![" ATTENTION".to_string()];
    if explained.attention.is_empty() {
        lines.push("   nothing in sight".into());
    }
    for (thing, score) in &explained.attention {
        let marker = if explained.attended.as_ref() == Some(thing) {
            "►"
        } else {
            " "
        };
        lines.extend(scored(
            &format!(" {marker} "),
            &thing_name(thing, data),
            &level(*score),
        ));
    }
    lines.push(String::new());
    match explained.decision {
        Some((verb, score)) => {
            let head = format!("DECISION: {}", verb_name(verb));
            lines.extend(scored(" ", &head, &level(score)));
        }
        None => lines.push(" DECISION: none".into()),
    }
    // A part that rounds to nothing adds nothing worth showing.
    let shown = explained
        .contributions
        .iter()
        .filter(|c| level(c.amount.abs()) != ".00");
    for contribution in shown.take(CONCEPTS_SHOWN) {
        let amount = signed_level(contribution.amount);
        let name = match &contribution.part {
            Part::Concept(inputs) => concept_name(inputs),
            // Neutral, since worth adds to a verb or takes from it either
            // way: a bad thing's takes from going near it.
            Part::Worth(thing) => format!("worth: {}", thing_name(thing, data)),
            Part::Fear(thing) => format!("fear: {}", thing_name(thing, data)),
            Part::Habit(thing) => {
                let verb = explained.decision.map_or("", |(verb, _)| verb_name(verb));
                format!("habit: {} {}", verb.to_lowercase(), thing_name(thing, data))
            }
        };
        lines.extend(scored("   ", &name, &amount));
    }
    lines
}

/// Something learned, as the memory words it: `thornbushes are bad`,
/// `water is good for thirst`, `Sprite #7 is frightening`, `eating balls`,
/// `new things`.
pub(super) fn learned_name(learned: &Learned, data: &Words) -> String {
    match learned {
        Learned::Worth {
            thing,
            need: Some(need),
        } => {
            let (things, be) = things(thing, data);
            format!("{things} {be} good for {}", display_name(need))
        }
        Learned::Worth { thing, need: None } => {
            let (things, be) = things(thing, data);
            format!("{things} {be} good")
        }
        Learned::Bad { thing } => {
            let (things, be) = things(thing, data);
            format!("{things} {be} bad")
        }
        Learned::Fear { thing } => {
            let (things, be) = things(thing, data);
            format!("{things} {be} frightening")
        }
        Learned::Habit { thing, verb } => format!("{} {}", doing(*verb), things(thing, data).0),
        Learned::NewThings => "new things".into(),
    }
}

/// A thing as the Brain tab names it (design v19 §6.1): an object type or
/// a category by its display name, `berry bush`, a sprite as the log names
/// it, `Sprite #7`.
fn thing_name(thing: &Thing, data: &Words) -> String {
    match thing {
        Thing::ObjectType(name) | Thing::Category(name) => display_name(name),
        Thing::Sprite(id) => data.label(*id),
        Thing::Cursor => CURSOR.into(),
    }
}

/// A thing with the verb "to be" to go with it: an object type in general,
/// as it names itself (design §3.5.1), `thornbushes are`, `water is`; a
/// category's summary, as the category names itself (design v19 §6.1),
/// `bushes are`, `fruit is`; or a particular sprite, `Sprite #7 is` (design
/// v18 §6.1). One with no plural isn't counted, so it keeps its name and
/// takes "is".
fn things(thing: &Thing, data: &Words) -> (String, &'static str) {
    let counted = |name: &str, plural: Option<&str>| match plural {
        Some(plural) => (plural.to_string(), "are"),
        None => (display_name(name), "is"),
    };
    match thing {
        Thing::ObjectType(name) => counted(name, data.plural_of(name)),
        Thing::Category(name) => counted(name, data.category_plural(name)),
        Thing::Sprite(id) => (data.label(*id), "is"),
        Thing::Cursor => (CURSOR.into(), "is"),
    }
}

/// A verb as a habit words doing it to something: `eating`, `playing with`.
fn doing(verb: Verb) -> &'static str {
    match verb {
        Verb::Approach => "going to",
        Verb::Eat => "eating",
        Verb::Drink => "drinking from",
        Verb::Hit => "hitting",
        Verb::Play => "playing with",
        Verb::Retreat => "backing away from",
        Verb::Rest => "resting by",
        Verb::Wander => "wandering from",
        Verb::Mate => "mating with",
        Verb::BreedAlone => "breeding alone by",
        Verb::Speak => "speaking to",
        Verb::Court => "courting",
        Verb::Call => "calling by",
    }
}

/// A concept by its inputs, as the Brain tab and the Genome tab's instincts
/// word it: `hunger & not target adjacent`. A line wraps only between inputs.
pub(super) fn concept_name(inputs: &[(&str, bool)]) -> String {
    let inputs: Vec<String> = inputs
        .iter()
        .map(|&(input, negated)| match negated {
            true => unbroken(&format!("not {}", display_name(input))),
            false => unbroken(&display_name(input)),
        })
        .collect();
    inputs.join(" & ")
}

/// `name` after `head`, with `number` right-aligned a column short of the
/// inspector's edge; a name too long for the room left wraps at its words,
/// indented two columns past `head`.
pub(super) fn scored(head: &str, name: &str, number: &str) -> Vec<String> {
    let right = WIDTH - 1;
    // The first row leaves room for the number and a space before it; the
    // rest only for their indent.
    let room = right - head.chars().count() - number.chars().count() - 1;
    let later_room = right - head.chars().count() - 2;
    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    for word in name.split(' ') {
        let limit = if rows.is_empty() { room } else { later_room };
        if !row.is_empty() && row.chars().count() + 1 + word.chars().count() > limit {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    rows.push(row);
    let indent = " ".repeat(head.chars().count() + 2);
    let mut lines = Vec::new();
    for (i, text) in rows.iter().enumerate() {
        let text = text.replace(BOUND, " ");
        if i == 0 {
            let line = format!("{head}{text}");
            let gap = right.saturating_sub(line.chars().count() + number.chars().count());
            lines.push(format!("{line}{}{number}", " ".repeat(gap)));
        } else {
            lines.push(format!("{indent}{text}"));
        }
    }
    lines
}
