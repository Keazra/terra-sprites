//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.

use ratatui::style::{Color, Style};
use ratatui::text::Line;
use terra_sim::{
    ActionView, Blocker, ChemicalKind, ChemicalLevel, Command, CursorTouch, DeathCause, Dir,
    EmitterMode, Emptied, EntityId, Event, EventKind, Explanation, Expression, GeneView, Grip,
    Learned, MAX_NAME_CHARS, NameProblem, ObjectView, Outcome, Part, PlaceRule, Pos, Progress,
    Rejection, Removal, SpriteView, Target, Terrain, Thing, Trait, Verb, World,
};

use crate::app::{App, Selection, Tab};
use crate::policy::{Panel, Subject};
use crate::text;
use crate::text::{
    ROOTED, Words, cause_name, change, display_name, group_thousands, level, signed, signed_level,
    significant, terrain_name, whole,
};

/// The inspector's width, in columns, border included (design §6.1).
pub(crate) const INSPECTOR_WIDTH: u16 = 46;

/// The columns inside the inspector's border.
const WIDTH: usize = INSPECTOR_WIDTH as usize - 2;

/// What a sprite tab says with no sprite selected.
const NOTHING_SELECTED: &str = " No sprite selected: click one, or press Tab";

/// The inspector's title: the selected sprite, if any, then the tabs, with
/// the open one in brackets.
pub fn title(app: &App) -> String {
    let tabs: Vec<String> = Tab::ALL
        .iter()
        .map(|&tab| {
            if tab == app.tab() {
                format!("[{}]", tab.label())
            } else {
                tab.label().to_string()
            }
        })
        .collect();
    let tabs = tabs.join(" ");
    // The open tab's policy decides whether the title names the sprite
    // (design §6.4).
    let labels = app
        .selection()
        .map(|selection| selection.id())
        .filter(|&id| app.can_view(Panel::Tab(app.tab()), Subject::Sprite(id)))
        .map(|id| (app.names().label(id), format!("#{}", id.0)));
    fitted_title(
        labels
            .as_ref()
            .map(|(label, short)| (label.as_str(), short.as_str())),
        &tabs,
    )
}

/// The columns the border leaves the title: all but its corners and the
/// line before the title.
const TITLE_ROOM: usize = INSPECTOR_WIDTH as usize - 3;

/// The title with the sprite's `label` if it fits, or else its `short`
/// label, or else no label: the tabs are never cut (design §6.1).
fn fitted_title(labels: Option<(&str, &str)>, tabs: &str) -> String {
    let (label, short) = labels.unzip();
    [label, short]
        .into_iter()
        .flatten()
        .map(|label| format!(" {label} ── {tabs} "))
        .find(|title| title.chars().count() <= TITLE_ROOM)
        .unwrap_or_else(|| format!(" {tabs} "))
}

/// The open tab's lines, from the top.
pub fn lines(app: &App, world: &World) -> Vec<Line<'static>> {
    // The policy may hide a tab (design §6.4).
    let subject = match (app.tab(), app.selection()) {
        (Tab::World, _) | (_, None) => Subject::World,
        (_, Some(selection)) => Subject::Sprite(selection.id()),
    };
    // Saying how to select a sprite tells nothing about the world.
    let hint = app.tab() != Tab::World && app.selection().is_none();
    if !hint && !app.can_view(Panel::Tab(app.tab()), subject) {
        return Vec::new();
    }
    match (app.tab(), app.selection()) {
        (Tab::World, _) => world_tab(world),
        (_, None) => vec![Line::from(NOTHING_SELECTED)],
        (tab, Some(Selection::Living(id))) => match world.sprite(id) {
            Some(sprite) => sprite_tab(tab, &sprite, app, world),
            None => Vec::new(),
        },
        (_, Some(Selection::Dead { id, cause, age })) => {
            let how = match cause {
                DeathCause::HurtBy(_) => cause_name(cause, world.data()),
                _ => format!("of {}", cause_name(cause, world.data())),
            };
            let text = format!(
                "{} died {} at age{BOUND}{}",
                unbroken(&app.names().label(id)),
                unbroken(&how),
                group_thousands(age)
            );
            wrapped(&text, 1, Style::default())
        }
    }
}

/// The first line shown of a tab `length` lines long in `rows` rows, when
/// it's scrolled `scroll` lines: no further than where its last line comes
/// into view, since the tab may have got shorter, or the rows more.
pub(crate) fn first_shown(scroll: usize, length: usize, rows: usize) -> usize {
    scroll.min(length.saturating_sub(rows))
}

/// A sprite tab's lines for `sprite`, the selection.
fn sprite_tab(tab: Tab, sprite: &SpriteView, app: &App, world: &World) -> Vec<Line<'static>> {
    match tab {
        Tab::Body => body_tab(sprite, app, world),
        Tab::Brain => brain_tab(sprite, &app.words(world)),
        Tab::Chem => chem_tab(sprite),
        Tab::Genome => genome_tab(sprite),
        Tab::World => Vec::new(),
    }
}

/// The Body tab (design §6.1): what the sprite is doing, age and traits, a
/// bar for each drive, the physical levels, three to a line, and what the
/// player has observed it do.
fn body_tab(sprite: &SpriteView, app: &App, world: &World) -> Vec<Line<'static>> {
    let traits = sprite.traits();
    let doing = doing_line(sprite, app, world).map(|line| format!(" {line}"));
    let mut lines: Vec<String> = doing.into_iter().collect();
    lines.extend([
        format!(
            " age {} · {}",
            group_thousands(sprite.age()),
            trait_text(Trait::Lifespan, traits.lifespan)
        ),
        format!(
            " {} · {}",
            trait_text(Trait::Speed, traits.speed),
            trait_text(Trait::SenseRadius, traits.sense_radius)
        ),
        String::new(),
    ]);
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    for drive in chemicals.iter().filter(|c| c.kind == ChemicalKind::Drive) {
        let line = format!(
            " {:<12}{} {} {}",
            display_name(drive.name),
            bar(drive.level),
            level(drive.level),
            trend(drive.change)
        );
        lines.push(line.trim_end().to_string());
    }
    lines.push(String::new());
    let physical: Vec<String> = chemicals
        .iter()
        .filter(|c| c.kind == ChemicalKind::Physical)
        .map(|c| format!("{} {}", display_name(c.name), level(c.level)))
        .collect();
    for three in physical.chunks(3) {
        lines.push(format!(" {}", three.join(" · ")));
    }
    lines.push(String::new());
    lines.extend(observed_lines(app, world.tick()));
    lines.into_iter().map(Line::from).collect()
}

/// What `sprite` is doing, as the Body tab's first line says it (design
/// §6.1): sliding from a shove, being led, or its action; nothing if it has
/// no action. The sprite list's Doing column says the same.
pub(crate) fn doing_line(sprite: &SpriteView, app: &App, world: &World) -> Option<String> {
    slide_line(sprite, app.detail())
        .or_else(|| led_line(sprite, app.detail(), world))
        .or_else(|| {
            sprite
                .action()
                .map(|action| action_line(&action, app.detail(), &app.words(world)))
        })
}

/// The observed list (design §6.1), newest first: how long ago each line's
/// latest action finished, right-aligned, and what it did, wrapped under
/// its own text. `now` is the world's tick counter.
fn observed_lines(app: &App, now: u64) -> Vec<String> {
    let mut lines = vec![" Observed".to_string()];
    let entries: Vec<(String, String)> = app
        .observed()
        .map(|o| {
            // An action ending on tick t has been over since the world moved
            // on to t + 1.
            let ago = match now.saturating_sub(o.tick + 1) {
                0 => "just now".to_string(),
                1 => "1 tick ago".to_string(),
                n => format!("{} ticks ago", group_thousands(n)),
            };
            let times = if o.count > 1 {
                format!(" ×{}", o.count)
            } else {
                String::new()
            };
            (ago, format!("{}{times}", o.line))
        })
        .collect();
    if entries.is_empty() {
        lines.push("   nothing yet".into());
    }
    let width = entries.iter().map(|(ago, _)| ago.chars().count()).max();
    for (ago, text) in &entries {
        let head = format!(" {ago:>width$} · ", width = width.unwrap_or(0));
        lines.extend(hanging(&head, text));
    }
    lines
}

/// `text` after `head`, wrapped to the inspector's width with each later
/// line indented to where the text began.
fn hanging(head: &str, text: &str) -> Vec<String> {
    let indent = " ".repeat(head.chars().count());
    let mut lines = Vec::new();
    let mut line = head.to_string();
    let mut empty = true;
    for word in text.split(' ') {
        if !empty && line.chars().count() + 1 + word.chars().count() > WIDTH {
            lines.push(std::mem::replace(&mut line, indent.clone()));
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    lines.push(line);
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

/// What a sprite is doing, as the Body tab's first line says it (design
/// §6.1): in plain words, describing and never speaking as the sprite; or,
/// in the detail view, exactly, with its verb, destination or target, and
/// outcome.
fn action_line(action: &ActionView, detail: bool, data: &Words) -> String {
    let plain = match action.verb {
        _ if detail => None,
        Verb::Wander => wander_line(action.progress),
        Verb::Rest => rest_line(action.progress),
        _ => aimed_line(action, data),
    };
    plain.unwrap_or_else(|| exact_line(action, data))
}

/// An aimed action in plain words, naming what it's aimed at, if it has
/// words for `progress`.
fn aimed_line(action: &ActionView, data: &Words) -> Option<String> {
    action.target?;
    let what = target_words(action, data);
    // Out of sight, it goes by what it remembers (M2 design §7).
    let aim = match action.remembered {
        true => format!("{what} it remembers"),
        false => what.clone(),
    };
    let going = match action.verb {
        Verb::Eat => format!("Going to eat {aim}"),
        Verb::Drink if action.remembered => format!("Going to drink at {aim}"),
        Verb::Drink => "Going to drink".into(),
        Verb::Approach => format!("Going over to {aim}"),
        Verb::Play => format!("Going to play with {aim}"),
        Verb::Hit => format!("Going to hit {aim}"),
        Verb::Retreat => format!("Backing away from {what}"),
        _ => return None,
    };
    Some(match action.progress {
        Progress::Walking { steps_left } => {
            format!(
                "{going} · {} to go",
                counted(steps_left, walked(action.verb))
            )
        }
        Progress::Waiting { .. } if action.verb == Verb::Retreat => {
            format!("{going} · waiting for room")
        }
        Progress::Waiting { .. } => format!("{going} · waiting to get past"),
        Progress::Resting { .. } => return None,
        Progress::Ended(Outcome::Applied) => done_line(action, &what, data),
        Progress::Ended(Outcome::Failed) if action.attempted => {
            let couldnt = match action.verb {
                Verb::Eat => format!("Couldn't eat from {what}"),
                Verb::Play => format!("Couldn't play with {what}"),
                Verb::Hit => format!("Couldn't hit {what}"),
                _ => "Couldn't drink".into(),
            };
            and_if_hurt(couldnt, action)
        }
        Progress::Ended(Outcome::Failed) if action.target_gone => {
            format!("Gave up: {what} was gone")
        }
        Progress::Ended(Outcome::Blocked) if action.verb == Verb::Retreat => {
            "Backed into a corner".into()
        }
        Progress::Ended(outcome) => return ended_line(outcome),
    })
}

/// What a walking action counts down: a retreat, the steps of its bout;
/// any other, the tiles of its path.
fn walked(verb: Verb) -> &'static str {
    if verb == Verb::Retreat {
        "step"
    } else {
        "tile"
    }
}

/// A finished action in the past tense, for the Body tab's observed list
/// (design §6.1): what it did, or what it set out to do and how that went.
pub(crate) fn observed_line(action: &ActionView, data: &Words) -> String {
    let Progress::Ended(outcome) = action.progress else {
        return action_line(action, false, data);
    };
    let what = target_words(action, data);
    let set_out = match action.verb {
        Verb::Wander => "Wandered off".to_string(),
        Verb::Rest => "Rested".into(),
        Verb::Eat => format!("Went to eat {what}"),
        Verb::Drink => "Went to drink".into(),
        Verb::Approach => format!("Went over to {what}"),
        Verb::Play => format!("Went to play with {what}"),
        Verb::Hit => format!("Went to hit {what}"),
        Verb::Retreat => format!("Backed away from {what}"),
        verb => verb_name(verb).to_lowercase(),
    };
    let how = match outcome {
        Outcome::Applied => {
            return match action.verb {
                Verb::Wander | Verb::Rest | Verb::Approach | Verb::Retreat => set_out,
                _ => done_line(action, &what, data),
            };
        }
        Outcome::Blocked if action.verb == Verb::Retreat => "was cornered".into(),
        Outcome::Failed if action.attempted => match action.verb {
            Verb::Eat | Verb::Drink => "it was empty".to_string(),
            _ => "couldn't".into(),
        },
        Outcome::Failed if action.target_gone => match action.target {
            Some(Target::Sprite(_)) => format!("{what} was gone"),
            _ => "it was gone".into(),
        },
        Outcome::Interrupted => "changed its mind".into(),
        Outcome::PulledAway => "was pulled away".into(),
        outcome => {
            let reason = ended_line(outcome).expect("an outcome that isn't applied");
            reason.replacen("Gave up", "gave up", 1)
        }
    };
    and_if_hurt(format!("{set_out}, but {how}"), action)
}

/// What sprite `actor`'s finished `action` did to the sprite it was aimed
/// at, as that sprite's observed list says it (design §6.1): "Was hit by
/// Sprite #7". `None` if it did nothing worth a line.
pub(crate) fn done_to_line(actor: EntityId, action: &ActionView, data: &Words) -> Option<String> {
    if action.progress != Progress::Ended(Outcome::Applied) {
        return None;
    }
    let who = data.label(actor);
    match action.verb {
        Verb::Hit => Some(format!("Was hit by {who}")),
        Verb::Play => Some(format!("{who} played with it")),
        _ => None,
    }
}

/// What an aimed action that applied did to `what`, its target in words,
/// in the past tense: "Ate from the berry bush", "Kicked the ball", and
/// whether it got hurt doing it.
fn done_line(action: &ActionView, what: &str, data: &Words) -> String {
    let deed = deed(action, what, data);
    let mut letters = deed.chars();
    let first = letters.next().map(|c| c.to_ascii_uppercase());
    and_if_hurt(first.into_iter().chain(letters).collect(), action)
}

/// `line`, with ", and got hurt" if `action`'s attempt hurt its own sprite,
/// whatever hurt it.
fn and_if_hurt(line: String, action: &ActionView) -> String {
    if action.hurt.actor {
        format!("{line}, and got hurt")
    } else {
        line
    }
}

/// What an aimed action that applied did to `what`, its target in words,
/// in the past tense and lower case: "ate from the berry bush", "kicked the
/// ball". A kick is a Play that pushes, as the target's verb table says.
fn deed(action: &ActionView, what: &str, data: &Words) -> String {
    let pushes = |verb| action.target_type.is_some_and(|t| data.pushes(t, verb));
    match action.verb {
        // An Eat that hurts, a thornbush's say, gave no food.
        Verb::Eat if action.hurt.actor => format!("tried to eat {what}"),
        // A thing eaten whole is gone; one eaten from is still there.
        Verb::Eat if action.target_gone => format!("ate {what}"),
        Verb::Eat => format!("ate from {what}"),
        Verb::Drink => "drank".into(),
        Verb::Play if pushes(Verb::Play) => format!("kicked {what}"),
        Verb::Play => format!("played with {what}"),
        Verb::Hit => format!("hit {what}"),
        Verb::Retreat => format!("backed away from {what}"),
        _ => format!("got to {what}"),
    }
}

/// `name` after "a", or "an" before a vowel, whatever its case: "a ball",
/// "an Apple".
fn with_article(name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "aeiou".contains(c.to_ascii_lowercase()));
    format!("{} {name}", if vowel { "an" } else { "a" })
}

/// Sprite `actor`'s finished `action` as the event log says it (design
/// §6.1), if the log shows it: every Play and Hit that applied, and any
/// attempt that hurt a sprite, even one that then failed. "Sprite #4
/// kicked a ball".
pub(crate) fn logged_line(actor: EntityId, action: &ActionView, data: &Words) -> Option<String> {
    let applied = action.progress == Progress::Ended(Outcome::Applied);
    let hurt = action.hurt.actor || action.hurt.target;
    let played = applied && matches!(action.verb, Verb::Play | Verb::Hit);
    if !played && !(action.attempted && hurt) {
        return None;
    }
    let what = match action.target? {
        Target::Sprite(id) => data.label(id),
        Target::Water(_) => "the water".into(),
        Target::Cursor => CURSOR.into(),
        Target::Object(_) => {
            let name = action.target_type.and_then(|id| data.object_type_name(id));
            with_article(&display_name(name.unwrap_or("?")))
        }
    };
    let deed = deed(action, &what, data);
    let hurt_itself = if action.hurt.actor {
        " and got hurt"
    } else {
        ""
    };
    Some(format!("{} {deed}{hurt_itself}", data.label(actor)))
}

/// What an aimed action is aimed at, in words: "the berry bush", "the
/// water", "Sprite #530". Empty for an action aimed at nothing.
fn target_words(action: &ActionView, data: &Words) -> String {
    match action.target {
        Some(Target::Sprite(id)) => data.label(id),
        Some(Target::Water(_)) => "the water".into(),
        Some(Target::Cursor) => CURSOR.into(),
        Some(Target::Object(_)) => {
            let name = action.target_type.and_then(|id| data.object_type_name(id));
            format!("the {}", display_name(name.unwrap_or("?")))
        }
        None => String::new(),
    }
}

/// How any action that ended without doing what it set out to ends, in plain words.
fn ended_line(outcome: Outcome) -> Option<String> {
    Some(
        match outcome {
            Outcome::Applied => return None,
            Outcome::Blocked => "Gave up: the way was blocked",
            Outcome::TimedOut => "Gave up: it took too long",
            Outcome::Failed => "Gave up: it couldn't get there",
            Outcome::Interrupted => "Changed its mind",
            Outcome::PulledAway => "Pulled away",
        }
        .into(),
    )
}

/// A Wander in plain words, if it has any for `progress`.
fn wander_line(progress: Progress) -> Option<String> {
    Some(match progress {
        Progress::Walking { steps_left } => {
            format!("Wandering off · {} to go", counted(steps_left, "tile"))
        }
        Progress::Waiting { .. } => "Wandering off · waiting to get past".into(),
        Progress::Ended(Outcome::Applied) => "Arrived".into(),
        Progress::Ended(outcome) => return ended_line(outcome),
        Progress::Resting { .. } => return None,
    })
}

/// A Rest in plain words, if it has any for `progress`.
fn rest_line(progress: Progress) -> Option<String> {
    match progress {
        Progress::Resting { ticks, of } => Some(format!(
            "Resting · {} left",
            counted(of.saturating_sub(ticks), "tick")
        )),
        Progress::Ended(Outcome::Applied) => Some("Rested".into()),
        Progress::Ended(Outcome::Interrupted) => Some("Changed its mind".into()),
        _ => None,
    }
}

/// An action exactly: `WANDER → (61,40) · walking (5 tiles)`, or
/// `EAT → berry_bush #812 · applied`.
fn exact_line(action: &ActionView, data: &Words) -> String {
    let verb = verb_name(action.verb);
    let head = match (action.target, action.destination) {
        (Some(Target::Object(id)), _) => {
            let name = action.target_type.and_then(|t| data.object_type_name(t));
            format!("{verb} → {} #{}", name.unwrap_or("?"), id.0)
        }
        (Some(Target::Water(at)), _) => format!("{verb} → water ({},{})", at.x, at.y),
        (Some(Target::Sprite(id)), _) => format!("{verb} → sprite #{}", id.0),
        (Some(Target::Cursor), _) => format!("{verb} → cursor"),
        (None, Some(to)) => format!("{verb} → ({},{})", to.x, to.y),
        (None, None) => verb.to_string(),
    };
    let state = match action.progress {
        Progress::Walking { steps_left } => {
            format!("walking ({})", counted(steps_left, walked(action.verb)))
        }
        Progress::Waiting { blocked_ticks } => {
            format!("blocked ({})", counted(blocked_ticks, "tick"))
        }
        Progress::Resting { ticks, of } => format!("{ticks} of {of} ticks"),
        Progress::Ended(outcome) => outcome_name(outcome).to_string(),
    };
    // A trip to a remembered place (M2 design §7).
    let head = match action.remembered {
        true => format!("{head} · from memory"),
        false => head,
    };
    format!("{head} · {state}")
}

/// `n` of `thing`, pluralised: `1 tile`, `5 tiles`.
fn counted(n: u32, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

/// A verb's name in the detail view.
fn verb_name(verb: Verb) -> &'static str {
    match verb {
        Verb::Approach => "APPROACH",
        Verb::Eat => "EAT",
        Verb::Drink => "DRINK",
        Verb::Hit => "HIT",
        Verb::Play => "PLAY",
        Verb::Retreat => "RETREAT",
        Verb::Rest => "REST",
        Verb::Wander => "WANDER",
        Verb::Mate => "MATE",
        Verb::Speak => "SPEAK",
    }
}

/// An outcome's name in the detail view, as the design names it (§5.5).
fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Applied => "applied",
        Outcome::Blocked => "blocked",
        Outcome::Failed => "failed",
        Outcome::Interrupted => "interrupted",
        Outcome::TimedOut => "timed_out",
        Outcome::PulledAway => "pulled_away",
    }
}

/// A level as a bar ten cells long, filled to the nearest tenth.
fn bar(level: f32) -> String {
    // `as` makes a NaN 0.
    let filled = ((level * 10.0).round() as usize).min(10);
    format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled))
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

/// A trait and its value: `speed 7.25`, `sense 9.5`, `lifespan 61,204`.
fn trait_text(which: Trait, value: f32) -> String {
    match which {
        Trait::Speed => format!("speed {}", significant(value)),
        Trait::SenseRadius => format!("sense {}", significant(value)),
        Trait::Lifespan => format!("lifespan {}", whole(f64::from(value))),
    }
}

/// How many concepts the Brain tab lists under the decision.
const CONCEPTS_SHOWN: usize = 5;

/// The Brain tab (design §5.9, §6.1): what the sprite attended to and
/// decided at its latest step 5, or "Nothing decided yet", or while led
/// "Being led: it decides nothing"; then its memory, which it can have
/// before it first decides.
fn brain_tab(sprite: &SpriteView, data: &Words) -> Vec<Line<'static>> {
    // Led, it decides nothing, so its last decision would mislead (design
    // v23 §2.4).
    let mut lines = match sprite.explain() {
        // Sliding, likewise (design v25 §2.4).
        _ if sprite.slide().is_some() => vec![" Shoved: it decides nothing".to_string()],
        _ if sprite.lead_steps_left().is_some() => {
            vec![" Being led: it decides nothing".to_string()]
        }
        Some(explained) => explained_lines(&explained, data),
        None => vec![" Nothing decided yet".to_string()],
    };
    // What has learned only a rounding's worth has nothing worth showing.
    let remembered: Vec<_> = sprite
        .memory()
        .into_iter()
        .filter(|m| level(m.amount.abs()) != ".00")
        .collect();
    let mut gap = true;
    if !remembered.is_empty() {
        lines.push(String::new());
        lines.push(" MEMORY".into());
        gap = false;
    }
    for memory in remembered {
        let amount = signed_level(memory.amount);
        lines.extend(scored("   ", &learned_name(&memory.learned, data), &amount));
    }
    // Its remembered places, best remembered first (M2 design §7): what,
    // how far and which way, and how well.
    let places: Vec<_> = sprite
        .remembered_places()
        .into_iter()
        .filter(|p| level(p.recall) != ".00")
        .collect();
    if !places.is_empty() {
        if gap {
            lines.push(String::new());
        }
        lines.push(" PLACES".into());
    }
    for place in places {
        let name = format!(
            "{} · {}",
            thing_name(&place.thing, data),
            how_far(sprite.pos(), place.at)
        );
        lines.extend(scored("   ", &name, &level(place.recall)));
    }
    lines.into_iter().map(Line::from).collect()
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
/// to, with its score, the attended one marked; then the verb chosen, with
/// its score, and the concepts adding most to it, largest first. A snapshot
/// always has a verb; "none" only guards against one that doesn't.
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

/// A lesson as the event log says it (design §6.1): "Sprite #12 learned:
/// thornbushes are bad", "… water is good for thirst", "… eating balls is
/// bad".
pub(crate) fn learned_line(id: EntityId, learned: &Learned, good: bool, data: &Words) -> String {
    let verdict = if good { "good" } else { "bad" };
    let what = match learned {
        Learned::Worth { .. } | Learned::Bad { .. } | Learned::Fear { .. } => {
            learned_name(learned, data)
        }
        Learned::Habit { .. } => format!("{} is {verdict}", learned_name(learned, data)),
        Learned::NewThings => format!("new things are {verdict}"),
    };
    format!("{} learned: {what}", data.label(id))
}

/// Something learned, as the memory words it: `thornbushes are bad`,
/// `water is good for thirst`, `Sprite #7 is frightening`, `eating balls`,
/// `new things`.
fn learned_name(learned: &Learned, data: &Words) -> String {
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

/// How the screen names the Cursor, which sprites learn about while they
/// can see it (design v29 §6.1): "the Cursor is frightening".
const CURSOR: &str = "the Cursor";

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
        Verb::Speak => "speaking to",
    }
}

/// A concept by its inputs, as the Brain tab and the Genome tab's instincts
/// word it: `hunger & not target adjacent`. A line wraps only between inputs.
fn concept_name(inputs: &[(&str, bool)]) -> String {
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
fn scored(head: &str, name: &str, number: &str) -> Vec<String> {
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

/// The Chem tab (design §6.1): each chemical on its own line with its level
/// and its change per tick, the physical chemicals, then the signal
/// chemicals, with what the sprite felt on the reward line; then the
/// hormones' levels, four to a line.
fn chem_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    let line = |c: &ChemicalLevel| {
        // Reward is used up every tick, so its line shows what learning
        // took in instead of a change: `last_r` (design §6.1).
        let amount = match c.name {
            "reward" => format!("felt {}", signed_level(sprite.felt())),
            _ => change(c.change).unwrap_or_default(),
        };
        let name = display_name(c.name);
        let text = format!(" {name:<13}{:>4}  {amount}", level(c.level));
        text.trim_end().to_string()
    };
    let of_kinds = |kinds: &[ChemicalKind]| {
        chemicals
            .iter()
            .filter(|c| kinds.contains(&c.kind))
            .collect::<Vec<_>>()
    };
    let mut lines: Vec<String> = of_kinds(&[ChemicalKind::Physical])
        .into_iter()
        .map(line)
        .collect();
    lines.push(String::new());
    lines.extend(
        of_kinds(&[ChemicalKind::Drive, ChemicalKind::LearningSignal])
            .into_iter()
            .map(line),
    );
    lines.push(" HORMONES".into());
    for four in of_kinds(&[ChemicalKind::Hormone]).chunks(4) {
        let cells: Vec<String> = four
            .iter()
            .map(|c| format!("{:<4}{:>4}", display_name(c.name), level(c.level)))
            .collect();
        lines.push(format!(" {}", cells.join("   ")));
    }
    lines.into_iter().map(Line::from).collect()
}

/// A group of genes on the Genome tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneGroup {
    Traits,
    HalfLives,
    Reactions,
    Emitters,
    Receptors,
    StartingLevels,
    BrainSettings,
    Instincts,
    Attention,
    Unknown,
}

impl GeneGroup {
    /// Every group, in the tab's order. Traits come first, since they share
    /// one line.
    const ALL: [GeneGroup; 10] = [
        GeneGroup::Traits,
        GeneGroup::HalfLives,
        GeneGroup::Reactions,
        GeneGroup::Emitters,
        GeneGroup::Receptors,
        GeneGroup::StartingLevels,
        GeneGroup::BrainSettings,
        GeneGroup::Instincts,
        GeneGroup::Attention,
        GeneGroup::Unknown,
    ];

    /// The group `gene` goes in.
    fn of(gene: &GeneView) -> GeneGroup {
        match gene {
            GeneView::Trait { .. } => GeneGroup::Traits,
            GeneView::HalfLife { .. } => GeneGroup::HalfLives,
            GeneView::Reaction { .. } => GeneGroup::Reactions,
            GeneView::Emitter { .. } => GeneGroup::Emitters,
            GeneView::Receptor { .. } => GeneGroup::Receptors,
            GeneView::InitialConcentration { .. } => GeneGroup::StartingLevels,
            GeneView::BrainParam { .. } => GeneGroup::BrainSettings,
            GeneView::Instinct { .. } => GeneGroup::Instincts,
            GeneView::AttentionInstinct { .. } => GeneGroup::Attention,
            GeneView::Unknown { .. } => GeneGroup::Unknown,
        }
    }

    /// The group's heading.
    fn heading(self) -> &'static str {
        match self {
            GeneGroup::Traits => "TRAITS",
            GeneGroup::HalfLives => "HALF-LIVES",
            GeneGroup::Reactions => "REACTIONS",
            GeneGroup::Emitters => "EMITTERS",
            GeneGroup::Receptors => "RECEPTORS",
            GeneGroup::StartingLevels => "STARTING LEVELS",
            GeneGroup::BrainSettings => "BRAIN SETTINGS",
            GeneGroup::Instincts => "INSTINCTS",
            GeneGroup::Attention => "ATTENTION INSTINCTS",
            GeneGroup::Unknown => "UNKNOWN GENES",
        }
    }
}

/// The Genome tab (design §6.1): the genes grouped under headings, each
/// group in genome order, as plain lines. The expressed traits share one
/// line. A gene with no effect is dimmed, with the reason below it.
fn genome_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let genes = sprite.genes();
    let mut lines = Vec::new();
    for group in GeneGroup::ALL {
        let members: Vec<&(GeneView, Expression)> = genes
            .iter()
            .filter(|(gene, _)| GeneGroup::of(gene) == group)
            .collect();
        if members.is_empty() {
            continue;
        }
        lines.push(Line::from(format!(" {}", group.heading())));
        let (together, apart): (Vec<_>, Vec<_>) = members.into_iter().partition(|(gene, how)| {
            matches!(gene, GeneView::Trait { .. }) && *how == Expression::Expressed
        });
        if !together.is_empty() {
            let traits: Vec<String> = together.iter().map(|(gene, _)| gene_text(gene)).collect();
            lines.extend(wrapped(&listed(&traits), 1, Style::default()));
        }
        for (gene, how) in apart {
            let reason = match how {
                Expression::Expressed => {
                    lines.extend(wrapped(&gene_text(gene), 1, Style::default()));
                    continue;
                }
                Expression::Flagged(reason) => format!("flagged: {reason}"),
                Expression::Unexpressed => "unexpressed: an earlier gene sets this".into(),
                Expression::Unknown => "unknown: this version can't read it".into(),
                Expression::Unmatched => {
                    "unmatched: names a category this world doesn't have".into()
                }
            };
            let dim = Style::default().fg(Color::DarkGray);
            lines.extend(wrapped(&gene_text(gene), 1, dim));
            lines.extend(wrapped(&reason, 3, dim));
        }
    }
    lines
}

/// Keeps a word with the number after it when a line wraps.
const BOUND: char = '\u{a0}';

/// A gene as a plain line: `low energy → hunger +.00428 past .5`.
fn gene_text(gene: &GeneView) -> String {
    let past = |threshold: f32| {
        if threshold == 0.0 {
            String::new()
        } else {
            format!(" past{BOUND}{}", significant(threshold))
        }
    };
    match *gene {
        GeneView::Trait { which, value } => trait_text(which, value),
        GeneView::HalfLife { chem, ticks: 1 } => {
            format!("{} halves every tick", display_name(chem))
        }
        GeneView::HalfLife { chem, ticks } => format!(
            "{} halves every {} ticks",
            display_name(chem),
            group_thousands(u64::from(ticks))
        ),
        GeneView::Reaction {
            ref reactants,
            ref products,
            rate,
        } => {
            let side = |terms: &[(&str, u8)]| {
                if terms.is_empty() {
                    return "nothing".to_string();
                }
                let terms: Vec<String> = terms
                    .iter()
                    .map(|&(chem, n)| match n {
                        1 => display_name(chem),
                        n => format!("{n} {}", display_name(chem)),
                    })
                    .collect();
                terms.join(" + ")
            };
            format!(
                "{} → {}, rate{BOUND}{}",
                side(reactants),
                side(products),
                significant(rate)
            )
        }
        GeneView::Emitter {
            locus,
            mode,
            invert,
            threshold,
            gain,
            chem,
        } => {
            let locus = display_name(locus);
            let source = match mode {
                EmitterMode::Level if invert => format!("low {locus}"),
                EmitterMode::Level => locus,
                EmitterMode::Rise => format!("{locus} rises"),
                EmitterMode::Fall => format!("{locus} falls"),
            };
            format!(
                "{source} → {} {}{}",
                display_name(chem),
                signed(gain),
                past(threshold)
            )
        }
        GeneView::Receptor {
            chem,
            threshold,
            gain,
            target,
        } => format!(
            "{}{} → {} {}",
            display_name(chem),
            past(threshold),
            display_name(target),
            signed(gain)
        ),
        GeneView::BrainParam { param, value } => {
            format!("{} {}", display_name(param), significant(value))
        }
        GeneView::Instinct {
            ref inputs,
            verb,
            weight,
        } => format!(
            "{} → {} {}",
            concept_name(inputs),
            verb_name(verb).to_lowercase(),
            signed(weight)
        ),
        GeneView::AttentionInstinct {
            input,
            category,
            weight,
        } => format!(
            "{} → attends to {} {}",
            display_name(input),
            display_name(category),
            signed(weight)
        ),
        GeneView::InitialConcentration { chem, value } => {
            format!(
                "{} starts at{BOUND}{}",
                display_name(chem),
                significant(value)
            )
        }
        GeneView::Unknown {
            type_id,
            version,
            bytes: 1,
        } => format!("type {type_id}, version {version}, 1 byte"),
        GeneView::Unknown {
            type_id,
            version,
            bytes,
        } => format!("type {type_id}, version {version}, {bytes} bytes"),
    }
}

/// `text` with its spaces kept together when a line wraps.
fn unbroken(text: &str) -> String {
    text.replace(' ', &BOUND.to_string())
}

/// `items` as one line, `a · b · c`, which wraps only after a dot, never
/// inside an item.
fn listed(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|item| unbroken(item)).collect();
    items.join(&format!("{BOUND}· "))
}

/// `text` wrapped at word boundaries to fit the inspector, its first line
/// indented `indent` columns and the rest 3, all in `style`.
fn wrapped(text: &str, indent: usize, style: Style) -> Vec<Line<'static>> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = " ".repeat(indent);
    let mut empty = true;
    for word in text.split(' ') {
        // A word after another needs a space before it. The first word on a
        // line goes there whether it fits or not.
        if !empty && line.chars().count() + 1 + word.chars().count() > WIDTH {
            lines.push(std::mem::replace(&mut line, " ".repeat(3)));
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    lines.push(line);
    lines
        .into_iter()
        .map(|line| Line::styled(line.replace(BOUND, " "), style))
        .collect()
}

/// The World tab (design §6.1): the data pack; the population, and the
/// deaths by cause; then each object type with its count, the count in each
/// stage (for a type with more than one) and the total of each counter.
fn world_tab(world: &World) -> Vec<Line<'static>> {
    let data = world.data();
    // Physiology's causes always; an object only once it has killed.
    let hurt = world
        .deaths_by_cause()
        .map(|(cause, _)| cause)
        .filter(|cause| !DeathCause::PHYSIOLOGY.contains(cause));
    let causes: Vec<DeathCause> = DeathCause::PHYSIOLOGY.into_iter().chain(hurt).collect();
    let deaths: Vec<String> = causes
        .iter()
        .map(|&cause| {
            let n = world.deaths(cause);
            format!("{} {}", cause_name(cause, data), group_thousands(n))
        })
        .collect();
    let total: u64 = world.deaths_by_cause().map(|(_, n)| n).sum();
    let plain = Style::default();
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(format!(
            " {:<12}{} v{}",
            "data pack",
            data.name(),
            data.version()
        )),
        Line::from(format!(
            " {:<12}{:>7}",
            "sprites",
            group_thousands(world.sprites().count() as u64)
        )),
        Line::from(format!(" {:<12}{:>7}", "deaths", group_thousands(total))),
    ];
    lines.extend(wrapped(&listed(&deaths), 3, plain));
    for object_type in data.object_type_names() {
        let objects: Vec<ObjectView> = world
            .objects()
            .filter(|o| o.type_name() == object_type)
            .collect();
        lines.push(Line::from(format!(
            " {:<12}{:>7}",
            display_name(object_type),
            group_thousands(objects.len() as u64)
        )));
        let stages = data.stage_names(object_type);
        if stages.len() > 1 {
            let counts: Vec<String> = stages
                .iter()
                .map(|&stage| {
                    let n = objects.iter().filter(|o| o.stage() == Some(stage)).count();
                    format!("{stage} {}", group_thousands(n as u64))
                })
                .collect();
            lines.extend(wrapped(&listed(&counts), 3, plain));
        }
        let totals: Vec<String> = data
            .counter_names(object_type)
            .iter()
            .map(|&counter| {
                let total: u64 = objects
                    .iter()
                    .map(|o| u64::from(o.counter(counter).unwrap_or(0)))
                    .sum();
                format!("{counter} {}", group_thousands(total))
            })
            .collect();
        if !totals.is_empty() {
            lines.extend(wrapped(&listed(&totals), 3, plain));
        }
    }
    lines
}

/// Why the world refused `command`, spoken to the player, as the event log
/// and the status line say it (design v22, v23 §6.1): "Couldn't pet Sprite
/// #12: it's gone", "Couldn't put the ball down: a berry is there",
/// "Couldn't throw the ball: a berry is there" (design v25).
fn refusal_line(command: &Command, reason: Rejection, data: &Words) -> Option<String> {
    let name = |id: u16| display_name(data.object_type_name(id).unwrap_or("?"));
    let what = match command.clone() {
        Command::Reward { sprite, .. } | Command::Correct { sprite, .. } => {
            let touch = CursorTouch::of(command).expect("a touch");
            format!("{} {}", touch.name(), data.label(sprite))
        }
        Command::TakeHold { sprite } => format!("take hold of {}", data.label(sprite)),
        Command::PickUp { .. } => "pick it up".into(),
        Command::PutDown { .. } => match reason {
            Rejection::InTheWay { item_type, .. } => format!("put the {} down", name(item_type)),
            _ => "put it down".into(),
        },
        Command::LetGo => "let go".into(),
        // The app sends it only while leading or visible, and only onto
        // the map; and the world never refuses showing or hiding it.
        Command::MoveCursor { .. } | Command::ShowCursor { .. } => return None,
        // Refused as putting it down would be (design v25 §2.5).
        Command::Throw { .. } => match reason {
            Rejection::InTheWay { item_type, .. } => format!("throw the {}", name(item_type)),
            _ => "throw it".into(),
        },
        Command::Shove { .. } => "shove".into(),
        Command::Place { object_type, .. } => format!("place the {}", name(object_type)),
        Command::SpawnSprite { .. } => "put a new sprite there".into(),
        Command::Rename { sprite, .. } => format!("name {}", data.label(sprite)),
    };
    let why = match reason {
        Rejection::Gone => "it's gone".into(),
        Rejection::Busy(Grip::Leads(led)) => {
            format!("you're already leading {}", data.label(led))
        }
        Rejection::Busy(Grip::Holds(_)) => "you're already holding something".into(),
        Rejection::NotLeading => "you're not leading a sprite".into(),
        Rejection::Rooted => ROOTED.into(),
        Rejection::NotHolding => "you're not holding anything".into(),
        Rejection::OffTheMap => "that's off the map".into(),
        Rejection::InTheWay { blocker, .. } | Rejection::NoRoom(blocker) => match blocker {
            Blocker::Object(there) => format!("{} is there", with_article(&name(there))),
            Blocker::Sprite => "a sprite is there".into(),
            Blocker::Terrain(terrain) => {
                let into = match terrain {
                    Terrain::ShallowWater | Terrain::DeepWater => "in",
                    _ => "on",
                };
                format!("it can't go {into} {}", terrain_name(terrain))
            }
        },
        Rejection::NotPlaceable => "the Cursor can't make one of those".into(),
        Rejection::PlaceRule { rule, .. } => match rule {
            PlaceRule::KeepsPathsOpen => "it would block the way".into(),
            PlaceRule::Fertility => "the ground there doesn't suit it".into(),
            PlaceRule::DensityBelow(crowding) => {
                let crowd = data.object_type_name(crowding).unwrap_or("?");
                let crowd = data
                    .plural_of(crowd)
                    .map_or_else(|| display_name(crowd), str::to_string);
                format!("there are too many {crowd} near")
            }
            PlaceRule::Itself => "it can't go there".into(),
        },
        Rejection::BadName(NameProblem::Empty) => "a name needs a letter in it".into(),
        Rejection::BadName(NameProblem::TooLong) => {
            format!("a name has at most {MAX_NAME_CHARS} letters")
        }
        Rejection::BadName(NameProblem::NotCp437) => {
            "the game can't show some of its letters".into()
        }
        Rejection::BadGenome => "its genome doesn't fit this world".into(),
    };
    Some(format!("Couldn't {what}: {why}"))
}

/// What an event says in the event log, if the log shows it (design §6.1).
pub(crate) fn event_line(event: &Event, data: &Words) -> Option<String> {
    match &event.kind {
        EventKind::Died {
            id,
            name,
            cause,
            age,
        } => Some(format!(
            "{} died ({}, age {})",
            name.as_deref()
                .map_or_else(|| data.label(*id), |name| text::label(*id, Some(name))),
            cause_name(*cause, data),
            group_thousands(*age)
        )),
        EventKind::ActionEnded { id, action, .. } => logged_line(*id, action, data),
        EventKind::LearnedMilestone { id, learned, good } => {
            Some(learned_line(*id, learned, *good, data))
        }
        // The Cursor's touch, spoken to the player (design v21 §6.1).
        EventKind::Rewarded { id, .. } | EventKind::Corrected { id, .. } => {
            let touch = CursorTouch::reported(&event.kind).expect("the Cursor's touch");
            let done = match touch {
                CursorTouch::Pet => "petted",
                CursorTouch::Hug => "hugged",
                CursorTouch::Zap => "zapped",
                CursorTouch::Shock => "shocked",
            };
            Some(format!("You {done} {}", data.label(*id)))
        }
        EventKind::CommandRejected { command, reason } => refusal_line(command, *reason, data),
        // Grabbing, spoken to the player (design v23 §6.1).
        EventKind::TookHold { sprite } => Some(format!("You took hold of {}", data.label(*sprite))),
        EventKind::LetGo { sprite } => Some(format!("You let go of {}", data.label(*sprite))),
        EventKind::PickedUp { object_type, .. } => Some(format!(
            "You picked up {}",
            with_article(&display_name(object_type))
        )),
        EventKind::PutDown { object_type, .. } => {
            Some(format!("You put the {} down", display_name(object_type)))
        }
        EventKind::CursorEmptied {
            reason:
                Emptied::Removed {
                    object_type,
                    reason,
                    ..
                },
        } => {
            let went = match reason {
                Removal::Expired => "expired",
                Removal::Destroyed => "was destroyed",
                Removal::Replaced => "was replaced",
            };
            let name = display_name(object_type);
            Some(format!("The {name} you were holding {went}"))
        }
        // A led sprite's death has its own line.
        EventKind::CursorEmptied {
            reason: Emptied::Died { .. },
        } => None,
        EventKind::ObjectSpawned { .. }
        | EventKind::ObjectRemoved { .. }
        | EventKind::ActionStarted { .. } => None,
        // Throwing and shoving, spoken to the player, and a crash only if it
        // hurt (design v25 §6.1).
        EventKind::Threw { object_type, .. } => {
            Some(format!("You threw the {}", display_name(object_type)))
        }
        EventKind::Shoved { sprite } => Some(format!("You shoved {}", data.label(*sprite))),
        EventKind::Crashed {
            sprite,
            into,
            hurt: true,
        } => Some(format!(
            "{} was shoved into {} and got hurt",
            data.label(*sprite),
            crashed_into(into, data)
        )),
        EventKind::Crashed { hurt: false, .. } => None,
        // The Place menu and naming, spoken to the player (design v28 §6.5).
        EventKind::Placed { object_type, .. } => Some(format!(
            "You placed {}",
            with_article(&display_name(object_type))
        )),
        EventKind::Spawned { id, .. } => Some(format!("You made {}", data.label(*id))),
        EventKind::Renamed { id, name } => Some(format!("You named Sprite #{} {name}", id.0)),
    }
}

/// The sprites an event is about, first the one it's mostly about: who
/// did it, then who it was done to. Empty for an event about no sprite.
pub(crate) fn event_sprites(event: &Event) -> Vec<EntityId> {
    match &event.kind {
        EventKind::Died { id, .. }
        | EventKind::LearnedMilestone { id, .. }
        | EventKind::Rewarded { id, .. }
        | EventKind::Corrected { id, .. }
        | EventKind::Spawned { id, .. }
        | EventKind::Renamed { id, .. }
        | EventKind::ActionStarted { id, .. } => vec![*id],
        EventKind::ActionEnded { id, action, .. } => match action.target {
            Some(Target::Sprite(target)) => vec![*id, target],
            _ => vec![*id],
        },
        EventKind::TookHold { sprite }
        | EventKind::LetGo { sprite }
        | EventKind::Shoved { sprite } => vec![*sprite],
        EventKind::Crashed { sprite, into, .. } => match into {
            Thing::Sprite(other) => vec![*sprite, *other],
            _ => vec![*sprite],
        },
        EventKind::CursorEmptied {
            reason: Emptied::Died { sprite },
        } => vec![*sprite],
        EventKind::CommandRejected { command, .. } => match command {
            Command::Reward { sprite, .. }
            | Command::Correct { sprite, .. }
            | Command::TakeHold { sprite }
            | Command::Rename { sprite, .. } => vec![*sprite],
            _ => Vec::new(),
        },
        EventKind::ObjectSpawned { .. }
        | EventKind::ObjectRemoved { .. }
        | EventKind::Placed { .. }
        | EventKind::PickedUp { .. }
        | EventKind::PutDown { .. }
        | EventKind::Threw { .. }
        | EventKind::CursorEmptied { .. } => Vec::new(),
    }
}

/// What a sprite crashed into, as a sentence says it: "a thornbush", or
/// "Sprite #7" (design v25 §6.1).
pub(crate) fn crashed_into(into: &Thing, data: &Words) -> String {
    match into {
        Thing::ObjectType(name) | Thing::Category(name) => with_article(&display_name(name)),
        Thing::Sprite(id) => data.label(*id),
        Thing::Cursor => CURSOR.into(),
    }
}

#[cfg(test)]
mod tests {
    use terra_sim::{Hurt, Pos};

    use super::*;

    // No sprite's number or name is long enough yet to crowd the tabs, so
    // this tests the title's fitting directly.
    #[test]
    fn a_label_too_long_for_the_title_is_shortened_before_the_tabs_are() {
        let tabs = "[Body] Chem Genome World";
        let fit = |label: &str, short: &str| fitted_title(Some((label, short)), tabs);
        assert_eq!(
            fit("Sprite #530", "#530"),
            " Sprite #530 ── [Body] Chem Genome World "
        );
        assert_eq!(
            fit("Sprite #1234567", "#1234567"),
            " #1234567 ── [Body] Chem Genome World "
        );
        let crowded = "[Body] Brain Chem Genome World Lineage";
        assert_eq!(
            fitted_title(Some(("Sprite #1234567", "#1234567")), crowded),
            format!(" {crowded} "),
            "with no room for even the number, the tabs alone"
        );
        assert_eq!(fitted_title(None, tabs), format!(" {tabs} "));
    }

    fn wander(progress: Progress) -> ActionView {
        ActionView {
            verb: Verb::Wander,
            destination: Some(Pos { x: 61, y: 40 }),
            target: None,
            target_type: None,
            attempted: false,
            target_gone: false,
            hurt: Hurt::default(),
            progress,
            remembered: false,
        }
    }

    fn rest(progress: Progress) -> ActionView {
        ActionView {
            verb: Verb::Rest,
            destination: None,
            target: None,
            target_type: None,
            attempted: false,
            target_gone: false,
            hurt: Hurt::default(),
            progress,
            remembered: false,
        }
    }

    // A timed-out action is followed by the next on the same tick, so the
    // screen rarely shows its ending: every wording is tested here instead.
    #[test]
    fn the_action_line_says_what_a_sprite_is_doing_plainly_or_exactly() {
        let cases = [
            (
                wander(Progress::Walking { steps_left: 5 }),
                "Wandering off · 5 tiles to go",
                "WANDER → (61,40) · walking (5 tiles)",
            ),
            (
                wander(Progress::Walking { steps_left: 1 }),
                "Wandering off · 1 tile to go",
                "WANDER → (61,40) · walking (1 tile)",
            ),
            (
                wander(Progress::Waiting { blocked_ticks: 2 }),
                "Wandering off · waiting to get past",
                "WANDER → (61,40) · blocked (2 ticks)",
            ),
            (
                wander(Progress::Ended(Outcome::Applied)),
                "Arrived",
                "WANDER → (61,40) · applied",
            ),
            (
                wander(Progress::Ended(Outcome::Blocked)),
                "Gave up: the way was blocked",
                "WANDER → (61,40) · blocked",
            ),
            (
                wander(Progress::Ended(Outcome::TimedOut)),
                "Gave up: it took too long",
                "WANDER → (61,40) · timed_out",
            ),
            (
                wander(Progress::Ended(Outcome::Failed)),
                "Gave up: it couldn't get there",
                "WANDER → (61,40) · failed",
            ),
            (
                rest(Progress::Resting { ticks: 4, of: 10 }),
                "Resting · 6 ticks left",
                "REST · 4 of 10 ticks",
            ),
            (
                rest(Progress::Resting { ticks: 9, of: 10 }),
                "Resting · 1 tick left",
                "REST · 9 of 10 ticks",
            ),
            (
                rest(Progress::Ended(Outcome::Applied)),
                "Rested",
                "REST · applied",
            ),
        ];
        for (view, plain, exact) in cases {
            assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
            assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
        }
        let nowhere = ActionView {
            destination: None,
            ..wander(Progress::Ended(Outcome::Failed))
        };
        assert_eq!(action_line(&nowhere, true, &pack()), "WANDER · failed");
    }

    /// The built-in pack's words, with no sprite named.
    fn pack() -> Words<'static> {
        let data = Box::leak(Box::new(
            terra_sim::DataPack::builtin().expect("built-in data pack is valid"),
        ));
        let names = Box::leak(Box::new(crate::text::Names::default()));
        Words { data, names }
    }

    /// An aimed action, `verb`, at `target` of the type `target_type`.
    fn aimed(verb: Verb, target: Target, target_type: u16, progress: Progress) -> ActionView {
        ActionView {
            verb,
            destination: None,
            target: Some(target),
            target_type: Some(target_type),
            attempted: false,
            target_gone: false,
            hurt: Hurt::default(),
            progress,
            remembered: false,
        }
    }

    #[test]
    fn the_action_line_says_a_trip_is_to_a_place_it_remembers() {
        // M2 design §7.
        let walking = Progress::Walking { steps_left: 34 };
        let remembered = |view: ActionView| ActionView {
            remembered: true,
            ..view
        };
        let water = aimed(
            Verb::Drink,
            Target::Water(Pos { x: 40, y: 12 }),
            100,
            walking,
        );
        let bush = aimed(Verb::Eat, Target::Object(EntityId(812)), 1, walking);
        let cases = [
            (
                remembered(water),
                "Going to drink at the water it remembers · 34 tiles to go",
                "DRINK → water (40,12) · from memory · walking (34 tiles)",
            ),
            (
                remembered(bush),
                "Going to eat the berry bush it remembers · 34 tiles to go",
                "EAT → berry_bush #812 · from memory · walking (34 tiles)",
            ),
        ];
        for (view, plain, exact) in cases {
            assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
            assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
        }
    }

    #[test]
    fn the_action_line_names_what_an_eat_drink_or_approach_is_aimed_at() {
        use Outcome::*;
        use Progress::*;
        // Object types: berry_bush 1, berry 2, water 100, sprite 101.
        let bush = |p| aimed(Verb::Eat, Target::Object(EntityId(812)), 1, p);
        let berry = |p| aimed(Verb::Eat, Target::Object(EntityId(9)), 2, p);
        let water = |p| aimed(Verb::Drink, Target::Water(Pos { x: 40, y: 12 }), 100, p);
        let sprite = |p| aimed(Verb::Approach, Target::Sprite(EntityId(530)), 101, p);
        let tried = |view: ActionView| ActionView {
            attempted: true,
            ..view
        };
        let gone = |view: ActionView| ActionView {
            target_gone: true,
            ..view
        };
        let cases = [
            (
                bush(Walking { steps_left: 3 }),
                "Going to eat the berry bush · 3 tiles to go",
                "EAT → berry_bush #812 · walking (3 tiles)",
            ),
            (
                bush(Waiting { blocked_ticks: 1 }),
                "Going to eat the berry bush · waiting to get past",
                "EAT → berry_bush #812 · blocked (1 tick)",
            ),
            (
                tried(bush(Ended(Applied))),
                "Ate from the berry bush",
                "EAT → berry_bush #812 · applied",
            ),
            (
                tried(bush(Ended(Failed))),
                "Couldn't eat from the berry bush",
                "EAT → berry_bush #812 · failed",
            ),
            (
                gone(tried(berry(Ended(Applied)))),
                "Ate the berry",
                "EAT → berry #9 · applied",
            ),
            (
                gone(berry(Ended(Failed))),
                "Gave up: the berry was gone",
                "EAT → berry #9 · failed",
            ),
            (
                bush(Ended(Failed)),
                "Gave up: it couldn't get there",
                "EAT → berry_bush #812 · failed",
            ),
            (
                bush(Ended(Interrupted)),
                "Changed its mind",
                "EAT → berry_bush #812 · interrupted",
            ),
            (
                water(Walking { steps_left: 2 }),
                "Going to drink · 2 tiles to go",
                "DRINK → water (40,12) · walking (2 tiles)",
            ),
            (
                tried(water(Ended(Applied))),
                "Drank",
                "DRINK → water (40,12) · applied",
            ),
            (
                sprite(Walking { steps_left: 4 }),
                "Going over to Sprite #530 · 4 tiles to go",
                "APPROACH → sprite #530 · walking (4 tiles)",
            ),
            (
                tried(sprite(Ended(Applied))),
                "Got to Sprite #530",
                "APPROACH → sprite #530 · applied",
            ),
            (
                gone(sprite(Ended(Failed))),
                "Gave up: Sprite #530 was gone",
                "APPROACH → sprite #530 · failed",
            ),
        ];
        for (view, plain, exact) in cases {
            assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
            assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
        }
    }

    /// `view` after its attempt, which hurt its own sprite.
    fn tried_and_hurt(view: ActionView) -> ActionView {
        let hurt = Hurt {
            actor: true,
            target: false,
        };
        ActionView {
            attempted: true,
            hurt,
            ..view
        }
    }

    #[test]
    fn play_and_hit_lines_say_kicked_for_a_push_and_got_hurt_for_a_hurt() {
        use Outcome::*;
        use Progress::*;
        // Object types: thornbush 3, ball 4, sprite 101.
        let ball = |verb, p| aimed(verb, Target::Object(EntityId(40)), 4, p);
        let thorns = |verb, p| aimed(verb, Target::Object(EntityId(77)), 3, p);
        let sprite = |verb, p| aimed(verb, Target::Sprite(EntityId(7)), 101, p);
        let tried = |view: ActionView| ActionView {
            attempted: true,
            ..view
        };
        let cases = [
            (
                ball(Verb::Play, Walking { steps_left: 3 }),
                "Going to play with the ball · 3 tiles to go",
                "PLAY → ball #40 · walking (3 tiles)",
            ),
            (
                tried(ball(Verb::Play, Ended(Applied))),
                "Kicked the ball",
                "PLAY → ball #40 · applied",
            ),
            (
                tried(ball(Verb::Hit, Ended(Applied))),
                "Hit the ball",
                "HIT → ball #40 · applied",
            ),
            (
                tried(sprite(Verb::Play, Ended(Applied))),
                "Played with Sprite #7",
                "PLAY → sprite #7 · applied",
            ),
            (
                sprite(Verb::Hit, Walking { steps_left: 1 }),
                "Going to hit Sprite #7 · 1 tile to go",
                "HIT → sprite #7 · walking (1 tile)",
            ),
            (
                tried(sprite(Verb::Hit, Ended(Applied))),
                "Hit Sprite #7",
                "HIT → sprite #7 · applied",
            ),
            (
                tried_and_hurt(thorns(Verb::Hit, Ended(Applied))),
                "Hit the thornbush, and got hurt",
                "HIT → thornbush #77 · applied",
            ),
            (
                tried_and_hurt(thorns(Verb::Play, Ended(Applied))),
                "Played with the thornbush, and got hurt",
                "PLAY → thornbush #77 · applied",
            ),
            (
                tried_and_hurt(thorns(Verb::Eat, Ended(Applied))),
                "Tried to eat the thornbush, and got hurt",
                "EAT → thornbush #77 · applied",
            ),
            (
                sprite(Verb::Play, Ended(Interrupted)),
                "Changed its mind",
                "PLAY → sprite #7 · interrupted",
            ),
            (
                tried(ball(Verb::Play, Ended(Failed))),
                "Couldn't play with the ball",
                "PLAY → ball #40 · failed",
            ),
            (
                tried(sprite(Verb::Hit, Ended(Failed))),
                "Couldn't hit Sprite #7",
                "HIT → sprite #7 · failed",
            ),
            (
                tried_and_hurt(thorns(Verb::Eat, Ended(Failed))),
                "Couldn't eat from the thornbush, and got hurt",
                "EAT → thornbush #77 · failed",
            ),
        ];
        for (view, plain, exact) in cases {
            assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
            assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
        }
    }

    #[test]
    fn play_and_hit_are_observed_in_the_past_tense_like_the_other_verbs() {
        use Outcome::*;
        use Progress::Ended;
        let ball = |verb, o| aimed(verb, Target::Object(EntityId(40)), 4, Ended(o));
        let thorns = |verb, o| aimed(verb, Target::Object(EntityId(77)), 3, Ended(o));
        let sprite = |verb, o| aimed(verb, Target::Sprite(EntityId(7)), 101, Ended(o));
        let tried = |view: ActionView| ActionView {
            attempted: true,
            ..view
        };
        let cases = [
            (tried(ball(Verb::Play, Applied)), "Kicked the ball"),
            (tried(sprite(Verb::Play, Applied)), "Played with Sprite #7"),
            (tried(sprite(Verb::Hit, Applied)), "Hit Sprite #7"),
            (
                tried_and_hurt(thorns(Verb::Eat, Applied)),
                "Tried to eat the thornbush, and got hurt",
            ),
            (
                tried_and_hurt(thorns(Verb::Play, Applied)),
                "Played with the thornbush, and got hurt",
            ),
            (
                sprite(Verb::Play, Interrupted),
                "Went to play with Sprite #7, but changed its mind",
            ),
            (
                ball(Verb::Hit, TimedOut),
                "Went to hit the ball, but gave up: it took too long",
            ),
            (
                tried(sprite(Verb::Hit, Failed)),
                "Went to hit Sprite #7, but couldn't",
            ),
            (
                tried_and_hurt(thorns(Verb::Eat, Failed)),
                "Went to eat the thornbush, but it was empty, and got hurt",
            ),
        ];
        for (view, line) in cases {
            assert_eq!(observed_line(&view, &pack()), line, "{view:?}");
        }
    }

    #[test]
    fn a_retreat_says_it_is_backing_away_and_whether_it_was_cornered() {
        use Outcome::*;
        use Progress::*;
        // Object types: thornbush 3, sprite 101.
        let sprite = |p| aimed(Verb::Retreat, Target::Sprite(EntityId(7)), 101, p);
        let thorns = |p| aimed(Verb::Retreat, Target::Object(EntityId(77)), 3, p);
        let gone = |view: ActionView| ActionView {
            target_gone: true,
            ..view
        };
        let cases = [
            (
                sprite(Walking { steps_left: 4 }),
                "Backing away from Sprite #7 · 4 steps to go",
                "RETREAT → sprite #7 · walking (4 steps)",
            ),
            (
                thorns(Walking { steps_left: 1 }),
                "Backing away from the thornbush · 1 step to go",
                "RETREAT → thornbush #77 · walking (1 step)",
            ),
            (
                sprite(Waiting { blocked_ticks: 2 }),
                "Backing away from Sprite #7 · waiting for room",
                "RETREAT → sprite #7 · blocked (2 ticks)",
            ),
            (
                sprite(Ended(Applied)),
                "Backed away from Sprite #7",
                "RETREAT → sprite #7 · applied",
            ),
            (
                sprite(Ended(Blocked)),
                "Backed into a corner",
                "RETREAT → sprite #7 · blocked",
            ),
            (
                gone(sprite(Ended(Failed))),
                "Gave up: Sprite #7 was gone",
                "RETREAT → sprite #7 · failed",
            ),
        ];
        for (view, plain, exact) in cases {
            assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
            assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
        }
        let observed = [
            (sprite(Ended(Applied)), "Backed away from Sprite #7"),
            (
                sprite(Ended(Blocked)),
                "Backed away from Sprite #7, but was cornered",
            ),
            (
                thorns(Ended(Interrupted)),
                "Backed away from the thornbush, but changed its mind",
            ),
        ];
        for (view, line) in observed {
            assert_eq!(observed_line(&view, &pack()), line, "{view:?}");
        }
    }

    #[test]
    fn a_name_takes_an_before_a_vowel_whatever_its_case() {
        assert_eq!(with_article("ball"), "a ball");
        assert_eq!(with_article("apple"), "an apple");
        assert_eq!(with_article("Apple"), "an Apple");
        assert_eq!(with_article("Egg"), "an Egg");
    }

    #[test]
    fn the_event_log_keeps_an_attempt_that_hurt_even_if_it_failed() {
        use Outcome::*;
        use Progress::Ended;
        let thorns = |verb, o| aimed(verb, Target::Object(EntityId(77)), 3, Ended(o));
        let me = EntityId(3);
        assert_eq!(
            logged_line(me, &tried_and_hurt(thorns(Verb::Eat, Failed)), &pack()).as_deref(),
            Some("Sprite #3 tried to eat a thornbush and got hurt")
        );
        let unhurt = ActionView {
            attempted: true,
            ..thorns(Verb::Hit, Failed)
        };
        assert_eq!(
            logged_line(me, &unhurt, &pack()),
            None,
            "a failed hit that hurt nobody"
        );
    }

    #[test]
    fn a_finished_action_is_observed_in_the_past_tense_saying_how_it_went_if_badly() {
        use Outcome::*;
        use Progress::Ended;
        let bush = |o| aimed(Verb::Eat, Target::Object(EntityId(812)), 1, Ended(o));
        let berry = |o| aimed(Verb::Eat, Target::Object(EntityId(9)), 2, Ended(o));
        let water = |o| {
            aimed(
                Verb::Drink,
                Target::Water(Pos { x: 4, y: 1 }),
                100,
                Ended(o),
            )
        };
        let sprite = |o| aimed(Verb::Approach, Target::Sprite(EntityId(530)), 101, Ended(o));
        let tried = |view: ActionView| ActionView {
            attempted: true,
            ..view
        };
        let gone = |view: ActionView| ActionView {
            target_gone: true,
            ..view
        };
        let cases = [
            (wander(Ended(Applied)), "Wandered off"),
            (
                wander(Ended(Blocked)),
                "Wandered off, but gave up: the way was blocked",
            ),
            (
                wander(Ended(TimedOut)),
                "Wandered off, but gave up: it took too long",
            ),
            (
                wander(Ended(Failed)),
                "Wandered off, but gave up: it couldn't get there",
            ),
            (rest(Ended(Applied)), "Rested"),
            (rest(Ended(Interrupted)), "Rested, but changed its mind"),
            (rest(Ended(PulledAway)), "Rested, but was pulled away"),
            (
                bush(PulledAway),
                "Went to eat the berry bush, but was pulled away",
            ),
            (tried(bush(Applied)), "Ate from the berry bush"),
            (gone(tried(berry(Applied))), "Ate the berry"),
            (
                tried(bush(Failed)),
                "Went to eat the berry bush, but it was empty",
            ),
            (
                bush(Interrupted),
                "Went to eat the berry bush, but changed its mind",
            ),
            (
                gone(berry(Failed)),
                "Went to eat the berry, but it was gone",
            ),
            (tried(water(Applied)), "Drank"),
            (
                water(Failed),
                "Went to drink, but gave up: it couldn't get there",
            ),
            (tried(sprite(Applied)), "Went over to Sprite #530"),
            (
                gone(sprite(Failed)),
                "Went over to Sprite #530, but Sprite #530 was gone",
            ),
        ];
        for (view, line) in cases {
            assert_eq!(observed_line(&view, &pack()), line, "{view:?}");
        }
    }

    #[test]
    fn a_scored_name_too_long_for_its_row_wraps_under_itself() {
        let name = concept_name(&[
            ("attended_sprite", true),
            ("target_adjacent", true),
            ("hunger", false),
        ]);
        let lines = scored("   ", &name, "+.40");
        assert_eq!(
            lines,
            [
                "   not attended sprite &               +.40",
                "     not target adjacent & hunger",
            ],
            "each input kept whole"
        );
        assert!(lines.iter().all(|l| l.chars().count() < WIDTH));
    }

    #[test]
    fn a_wrapped_line_has_the_room_the_number_leaves_on_the_first() {
        let name = concept_name(&[
            ("attended_sprite", true),
            ("target_distance", true),
            ("tiredness", true),
        ]);
        // The second row is 35 columns, which only fits because no number
        // follows it.
        assert_eq!(
            scored("   ", &name, "+.40"),
            [
                "   not attended sprite &               +.40",
                "     not target distance & not tiredness",
            ]
        );
    }

    // Levels never leave 0 to 1 (design §4.4), but a bar mustn't crash the
    // screen if one ever did.
    #[test]
    fn a_drive_bar_is_ten_cells_whatever_the_level() {
        assert_eq!(bar(0.42), "████░░░░░░");
        assert_eq!(bar(1.05), "██████████");
        assert_eq!(bar(-0.3), "░░░░░░░░░░");
        assert_eq!(bar(f32::NAN), "░░░░░░░░░░");
    }
}
