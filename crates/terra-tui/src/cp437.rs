//! The CP437 character set. All UI text stays within it, so any CP437 bitmap
//! font or tileset can draw every panel (design §6.2). The simulation knows
//! it too, since a sprite's name must fit it (design §2.5).

/// Whether CP437 can show `c`.
pub fn contains(c: char) -> bool {
    terra_sim::is_cp437(c)
}
