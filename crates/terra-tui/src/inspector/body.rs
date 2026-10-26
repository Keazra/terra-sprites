//! The Body tab (design §6.1): what the sprite is doing, its age and life
//! stage, its family, its traits and drives, and what the player has
//! observed it do; and the card every sprite tab shows for an egg (M2 design
//! §10.7).

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::{
    ChemicalKind, ChemicalLevel, EggView, EntityId, Fertility, LifeStage, Pos, SpriteView, Target,
    Trait, World,
};

use super::actions::action_line;
use super::{CURSOR, TabLine, WIDTH, counted, dotted, object_type};
use crate::app::App;
use crate::family_tree;
use crate::spots::Spot;
use crate::text::{Words, change, display_name, level, significant, thousands, whole, wrap};

/// The Body tab (design §6.1): what the sprite is doing, its age, life stage
/// and when it reaches the next (M2 design §9.6), its traits, a bar for each
/// drive, the physical levels, three to a line, with its wear (and its
/// genes' average health, once any has weakened) after them, and what the
/// player has observed it do.
pub(super) fn body_tab(sprite: &SpriteView, app: &App, world: &World) -> Vec<TabLine> {
    let look = app.theme.text();
    let line = |text: String| dotted(&text, Style::default(), look.muted);
    let traits = sprite.traits();
    let doing = doing_line(sprite, app, world).map(|line| format!(" {line}"));
    let mut lines: Vec<TabLine> = doing.into_iter().map(line).map(TabLine::from).collect();
    lines.extend(
        [
            format!(" age {} · {}", thousands(sprite.age()), {
                let stage = sprite.life_stage();
                match stage {
                    LifeStage::Young => {
                        format!("young · {}", trait_text(Trait::GrowsUp, traits.grows_up))
                    }
                    LifeStage::Adult => {
                        format!(
                            "adult · {}",
                            trait_text(Trait::GrowsOld, sprite.grows_old_at())
                        )
                    }
                    LifeStage::Old => stage.name().to_string(),
                }
            }),
            format!(
                " {} · {}",
                trait_text(Trait::Speed, traits.speed),
                trait_text(Trait::SenseRadius, traits.sense_radius)
            ),
        ]
        .map(line)
        .map(TabLine::from),
    );
    lines.extend(
        family_lines(sprite, &app.words(world))
            .into_iter()
            .map(line)
            .map(TabLine::from),
    );
    lines.push(Line::default().into());
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    for drive in chemicals.iter().filter(|c| c.kind == ChemicalKind::Drive) {
        // The bar filled in the drive's colour, as the map draws a sprite
        // it drives, and the rest of it minor (design §6.2).
        let filled = match app.theme.drive_colour(drive.name) {
            Some(colour) => Style::default().fg(colour),
            None => Style::default(),
        };
        let [full, empty] = bar_parts(drive.level);
        let mut spans = vec![
            Span::raw(format!(" {:<12}", display_name(drive.name))),
            Span::styled(full, filled),
            Span::styled(empty, look.muted),
            Span::raw(format!(" {}", level(drive.level))),
        ];
        let arrow = trend(drive.change);
        if !arrow.is_empty() {
            spans.push(Span::raw(format!(" {arrow}")));
        }
        // The pointer on it says what raises and lowers it (#304).
        let spot = Spot::Chemical {
            name: drive.name.to_string(),
            kind: drive.kind,
        };
        lines.push(TabLine::spotted(Line::from(spans), spot));
    }
    lines.push(Line::default().into());
    let physical: Vec<String> = chemicals
        .iter()
        .filter(|c| c.kind == ChemicalKind::Physical)
        .map(|c| format!("{} {}", display_name(c.name), level(c.level)))
        .collect();
    for three in physical.chunks(3) {
        lines.push(line(format!(" {}", three.join(" · "))).into());
    }
    // Its genes' average health shows once any has weakened (M2 design
    // §9.4, §9.6). A genome with nothing to weaken gives out when old
    // without any gene losing health, so that isn't shown.
    let weakened = sprite.gene_health().iter().any(|&health| health < 1.0);
    lines.push(TabLine::from(line(if weakened {
        format!(
            " wear {} · genes {}",
            level(sprite.wear()),
            level(sprite.vigour())
        )
    } else {
        format!(" wear {}", level(sprite.wear()))
    })));
    lines.push(Line::default().into());
    lines.extend(
        observed_lines(app, world.tick())
            .into_iter()
            .map(TabLine::from),
    );
    lines
}

/// What `sprite` is doing, as the Body tab's first line says it (design
/// §6.1): sliding from a shove, being led, standing with nothing to do, or
/// its action. The sprite list's Doing column says the same.
pub(crate) fn doing_line(sprite: &SpriteView, app: &App, world: &World) -> Option<String> {
    slide_line(sprite, app.detail())
        .or_else(|| led_line(sprite, app.detail(), world))
        .or_else(|| standing_line(sprite, app.detail(), &app.words(world), world))
        .or_else(|| {
            sprite
                .action()
                .map(|action| action_line(&action, app.detail(), &app.words(world)))
        })
}

/// While `sprite` stands with nothing to do, what the Body tab says in
/// place of an action (M2 design §19.2): what it's watching, "Watching Bo
/// #7", or "Standing" if nothing; in the detail view `STAND → sprite #7`.
fn standing_line(sprite: &SpriteView, detail: bool, data: &Words, world: &World) -> Option<String> {
    if !sprite.is_standing() {
        return None;
    }
    let attended = sprite.attended().zip(sprite.attending_to());
    Some(match (detail, attended) {
        (false, None) => "Standing".into(),
        (false, Some((target, at))) => format!("Watching {}", thing_words(target, at, data, world)),
        (true, None) => "STAND".into(),
        (true, Some((Target::Sprite(id), _))) => format!("STAND → sprite #{}", id.0),
        (true, Some((Target::Object(id), at))) => {
            format!("STAND → {} #{}", object_type(id, at, world), id.0)
        }
        (true, Some((Target::Water(at), _))) => format!("STAND → water ({},{})", at.x, at.y),
        (true, Some((Target::Cursor, _))) => "STAND → cursor".into(),
    })
}

/// A thing a sprite can attend to, on tile `at`, in words: "Bo #7", "the
/// berry bush", "the water".
fn thing_words(target: Target, at: Pos, data: &Words, world: &World) -> String {
    match target {
        Target::Sprite(id) => data.label(id),
        Target::Water(_) => "the water".into(),
        Target::Cursor => CURSOR.into(),
        Target::Object(id) => format!("the {}", display_name(object_type(id, at, world))),
    }
}

/// A sprite's sex and fertility, its parents, or where it came from
/// without, and its generation (M2 design §10, §17.5), and its partner, if it
/// has one (§15.8): `male · fertile · turning 40%`, `parents Pip #12 and Bo
/// #7 · generation 4`, `partner Kit #20`, or once its partner has died, until
/// it pairs again, `partner Kit #20 · died`. One whose body could breed but
/// for room says which room holds it back (§10.7, #331): `male · too
/// crowded to breed`, or `female · world full`.
fn family_lines(sprite: &SpriteView, data: &Words) -> Vec<String> {
    let mut line = format!(" {}", sprite.sex().name());
    if let Some(words) = fertility_words(sprite.fertility()) {
        line.push_str(&format!(" · {words}"));
    }
    let turning = sprite.turning().filter(|&share| share > 0.0);
    if let Some(share) = turning.filter(|_| sprite.life_stage().is_grown()) {
        line.push_str(&format!(" · turning {:.0}%", share * 100.0));
    }
    let mut lines = vec![line];
    let whence = match parents_text(sprite.parents(), data) {
        Some(parents) => format!("parents {parents}"),
        None => family_tree::origin_text(sprite.origin())
            .unwrap_or_default()
            .to_string(),
    };
    lines.extend(generation_lines(&whence, sprite.generation()));
    if let Some((partner, died)) = sprite.partner() {
        let died = if died { " · died" } else { "" };
        lines.push(format!(" partner {}{died}", data.label(partner)));
    }
    lines
}

/// What the Body tab says of a sprite's `fertility` (M2 design §10.7),
/// nothing for one whose body can't breed.
fn fertility_words(fertility: Fertility) -> Option<&'static str> {
    match fertility {
        Fertility::Fertile => Some("fertile"),
        Fertility::TooCrowded => Some("too crowded to breed"),
        Fertility::WorldFull => Some("world full"),
        Fertility::Infertile => None,
    }
}

/// Where a sprite or egg came from, `whence`, and its generation (M2
/// design §17.5): `parents Pip #12 and Bo #7 · generation 4`, on two lines if
/// that's too long for one.
fn generation_lines(whence: &str, generation: u32) -> Vec<String> {
    let line = format!(" {whence} · generation {generation}");
    if line.chars().count() <= WIDTH {
        vec![line]
    } else {
        vec![format!(" {whence}"), format!(" generation {generation}")]
    }
}

/// What every sprite tab shows for an egg (M2 design §10.7): when it
/// hatches, and who laid it, `egg · hatches in 750 ticks`, `parents Pip #12
/// and Bo #7`. One that's due waits while the Cursor holds it or a sprite
/// stands on it (§10.4).
pub(super) fn egg_card(
    egg: &EggView,
    world: &World,
    data: &Words,
    muted: Style,
) -> Vec<Line<'static>> {
    let when = match hatching(egg, world) {
        Hatching::OncePutDown => "hatches once it's put down".to_string(),
        Hatching::OnceClear => "hatches once nothing stands on it".to_string(),
        Hatching::In(left) => format!(
            "hatches in {}",
            counted(u32::try_from(left).unwrap_or(u32::MAX), "tick")
        ),
    };
    let mut lines = vec![format!(" {} · {when}", display_name(egg.type_name()))];
    if let Some(parents) = parents_text(egg.parents(), data) {
        lines.extend(generation_lines(
            &format!("parents {parents}"),
            egg.generation(),
        ));
    }
    lines
        .into_iter()
        .map(|line| dotted(&line, Style::default(), muted))
        .collect()
}

/// When an egg hatches (M2 design §10.4).
pub(crate) enum Hatching {
    /// In this many ticks, the one it hatches on included.
    In(u64),
    /// Once the Cursor puts it down: it's due, but held.
    OncePutDown,
    /// Once nothing stands on it: it's due, but a sprite does.
    OnceClear,
}

/// When `egg` hatches.
pub(crate) fn hatching(egg: &EggView, world: &World) -> Hatching {
    match ((egg.hatches() + 1).saturating_sub(world.tick()), egg.pos()) {
        (0, None) => Hatching::OncePutDown,
        (0, Some(_)) => Hatching::OnceClear,
        (left, _) => Hatching::In(left),
    }
}

/// Parents by name, `Pip #12 and Bo #7`, or `None` for none.
pub(super) fn parents_text(parents: &[EntityId], data: &Words) -> Option<String> {
    let names: Vec<String> = parents.iter().map(|&id| data.label(id)).collect();
    match names.as_slice() {
        [] => None,
        [one] => Some(one.clone()),
        [first, rest @ ..] => Some(format!("{first} and {}", rest.join(" and "))),
    }
}

/// The observed list (design §6.1), newest first: how long ago each line's
/// latest action finished, right-aligned, and what it did, wrapped under
/// its own text. `now` is the world's tick counter.
fn observed_lines(app: &App, now: u64) -> Vec<Line<'static>> {
    let look = app.theme.text();
    let mut lines = vec![Line::styled(" Observed", look.heading)];
    let entries: Vec<(String, String)> = app
        .observed()
        .map(|o| {
            // An action ending on tick t has been over since the world moved
            // on to t + 1.
            let ago = match now.saturating_sub(o.tick + 1) {
                0 => "just now".to_string(),
                1 => "1 tick ago".to_string(),
                n => format!("{} ticks ago", thousands(n)),
            };
            let times = if o.count > 1 {
                // An x: CP437 has no × (design §6.2).
                format!(" x{}", o.count)
            } else {
                String::new()
            };
            (ago, format!("{}{times}", o.line))
        })
        .collect();
    if entries.is_empty() {
        lines.push(Line::styled("   nothing yet", look.muted));
    }
    let width = entries.iter().map(|(ago, _)| ago.chars().count()).max();
    for (ago, text) in &entries {
        let head = format!(" {ago:>width$} · ", width = width.unwrap_or(0));
        // Each later line is indented to where the text began.
        let indent = " ".repeat(head.chars().count());
        lines.extend(
            wrap(&head, &indent, text, WIDTH)
                .into_iter()
                .map(|line| dotted(&line, Style::default(), look.muted)),
        );
    }
    lines
}

/// While a shove sends `sprite` sliding, what the Body tab says in place of
/// an action, even if the Cursor has taken hold of it meanwhile (design v25
/// §6.1): "Shoved · 2 tiles to go"; in the detail view
/// `SHOVED → NE · 2 tiles left`.
fn slide_line(sprite: &SpriteView, detail: bool) -> Option<String> {
    let (toward, left) = sprite.slide()?;
    let tiles = counted(u32::from(left), "tile");
    Some(if detail {
        format!("SHOVED → {toward:?} · {tiles} left")
    } else {
        format!("Shoved · {tiles} to go")
    })
}

/// While the Cursor leads `sprite`, what the Body tab says in place of an
/// action (design v23 §6.1): "Being led · 4 tiles behind", or once caught
/// up, as close as it can get, "Being led"; in the detail view
/// `LED → (61,40) · walking (4 tiles)`, with the Cursor's tile.
fn led_line(sprite: &SpriteView, detail: bool, world: &World) -> Option<String> {
    let behind = sprite.lead_steps_left()?;
    let tile = world.cursor().tile()?;
    Some(match (detail, behind) {
        (false, 0) => "Being led".into(),
        (false, _) => format!("Being led · {} behind", counted(behind, "tile")),
        (true, 0) => format!("LED → ({},{})", tile.x, tile.y),
        (true, _) => format!(
            "LED → ({},{}) · walking ({})",
            tile.x,
            tile.y,
            counted(behind, "tile")
        ),
    })
}

/// A level as a bar ten cells long, filled to the nearest tenth: its filled
/// part and its empty part.
pub(super) fn bar_parts(level: f32) -> [String; 2] {
    // `as` makes a NaN 0.
    let filled = ((level * 10.0).round() as usize).min(10);
    ["█".repeat(filled), "░".repeat(10 - filled)]
}

/// An arrow for which way a level went over the last tick, or nothing when
/// the change is too small to show.
fn trend(amount: f32) -> &'static str {
    match change(amount) {
        None => "",
        Some(_) if amount > 0.0 => "▲",
        Some(_) => "▼",
    }
}

/// A trait and its value: `speed 7.25`, `sense 9.5`, `grows old 13,500`.
fn trait_text(which: Trait, value: f32) -> String {
    format!("{} {}", trait_name(which), trait_value(which, value))
}

/// A trait's name on the screen: `speed`, `grows old`.
pub(super) fn trait_name(which: Trait) -> &'static str {
    match which {
        Trait::Speed => "speed",
        Trait::SenseRadius => "sense",
        Trait::GrowsOld => "grows old",
        Trait::GrowsUp => "grows up",
    }
}

/// A trait's value as the screen writes it: `7.25`, or an age in whole
/// ticks, `13,500`.
pub(super) fn trait_value(which: Trait, value: f32) -> String {
    match which {
        Trait::Speed | Trait::SenseRadius => significant(value),
        Trait::GrowsOld | Trait::GrowsUp => whole(f64::from(value)),
    }
}
