//! The CP437 character set. All UI text stays within it, so any CP437 bitmap
//! font or tileset can draw every panel (design §6.2).

/// The glyphs CP437 shows for bytes 0x01–0x1F and 0x7F–0xFF, in byte order.
/// Bytes 0x20–0x7E are printable ASCII.
const GLYPHS: &str = concat!(
    "☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼", // 0x01–0x1F
    "⌂",                               // 0x7F
    "ÇüéâäàåçêëèïîìÄÅ",                // 0x80
    "ÉæÆôöòûùÿÖÜ¢£¥₧ƒ",                // 0x90
    "áíóúñÑªº¿⌐¬½¼¡«»",                // 0xA0
    "░▒▓│┤╡╢╖╕╣║╗╝╜╛┐",                // 0xB0
    "└┴┬├─┼╞╟╚╔╩╦╠═╬╧",                // 0xC0
    "╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀",                // 0xD0
    "αßΓπΣσµτΦΘΩδ∞φε∩",                // 0xE0
    "≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}",           // 0xF0
);

/// Whether CP437 can show `c`.
pub fn contains(c: char) -> bool {
    matches!(c, ' '..='~') || GLYPHS.contains(c)
}
