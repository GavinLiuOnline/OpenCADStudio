// AutoCAD MIF escape decoding.
//
// AutoCAD embeds big-font (double-byte DBCS) characters as `\M+<hex>` escapes
// in drawing text — the ODA calls these "MIF codes". Native DWG/DXF readers
// surface these escapes verbatim, so without decoding a drawing's Chinese text
// renders as literal `\M+19094`-style garbage instead of characters.
//
// MIF code format (Aspose / ObjectARX `getMIFCodePage`):
//   - `\M+nXXXX` (5 hex digits): `n` is a 1-5 encoding index and `XXXX` is a
//     four-digit code in that encoding:
//       1 = Windows-31J (Shift-JIS), 2 = Big5, 3 = EUC-KR, 4 = Johab, 5 = GBK
//   - `\M+XXXX` (4 hex digits): a code in the drawing's own code page.
//   - `\U+XXXX`: Unicode (CIF) code, 4 hex digits.
//
// Escapes that do not form a valid character are left verbatim, so text that
// legitimately contains such sequences is never mangled.

use encoding_rs::{Encoding, BIG5, EUC_KR, GBK, SHIFT_JIS};

/// Pick the double-byte encoding a drawing's plain text uses, from the
/// drawing's code page (`$DWGCODEPAGE`). Falls back to GBK — the most common
/// Simplified-Chinese code page — for anything unrecognised. Used for 4-digit
/// `\M+XXXX` codes and, conceptually, for non-MIF text bytes.
fn encoding_for(code_page: &str) -> &'static Encoding {
    let c = code_page.to_ascii_uppercase();
    if c.contains("932") || c.contains("SJIS") || c.contains("SHIFT") {
        SHIFT_JIS
    } else if c.contains("949") || c.contains("KOR") || c.contains("UHC") {
        EUC_KR
    } else if c.contains("950") || c.contains("BIG5") {
        BIG5
    } else {
        GBK
    }
}

/// Encoding selected by the first digit of a 5-digit `\M+nXXXX` MIF code.
/// Johab (index 4) is not available in encoding_rs, so it returns `None` and
/// the escape falls through to the 4-digit path (or stays verbatim).
fn mif_encoding(index: u8) -> Option<&'static Encoding> {
    match index {
        b'1' => Some(SHIFT_JIS), // Windows-31J (cp932) — includes NEC/IBM extensions
        b'2' => Some(BIG5),
        b'3' => Some(EUC_KR),
        b'5' => Some(GBK),
        _ => None,
    }
}

/// Decode a two-byte code as a character in `enc`. Returns `None` when the
/// code is not a valid double-byte character (leading byte below 0x80, or an
/// unmapped byte pair).
fn decode_pair(enc: &'static Encoding, code: u16) -> Option<String> {
    let pair = [((code >> 8) & 0xFF) as u8, (code & 0xFF) as u8];
    // Double-byte codes always have a leading byte >= 0x80 in every DBCS
    // code page; a low byte pair would be a single-byte character.
    if pair[0] < 0x80 {
        return None;
    }
    let (decoded, _, had_errors) = enc.decode(&pair);
    if had_errors {
        return None;
    }
    Some(decoded.into_owned())
}

/// Decode AutoCAD MIF escapes in `text` into Unicode characters.
pub fn decode(text: &str, code_page: &str) -> String {
    let doc_enc = encoding_for(code_page);
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        // Any `\X+...` escape start.
        if bytes[i] == b'\\' && i + 2 < bytes.len() && bytes[i + 2] == b'+' {
            let kind = bytes[i + 1];
            if kind == b'M' || kind == b'm' {
                // Count the hex digits that follow (the escape is a fixed
                // width — 4 or 5 — so trailing hex-looking text is not eaten).
                let mut end = i + 3;
                while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
                    end += 1;
                }
                let hex = &text[i + 3..end];
                let n = hex.len();
                let mut decoded = None;
                // 5-digit: `\M+nXXXX` — encoding index + 4-digit code.
                if n >= 5 {
                    if let Some(enc) = mif_encoding(hex.as_bytes()[0]) {
                        if let Ok(v) = u32::from_str_radix(&hex[1..5], 16) {
                            decoded = decode_pair(enc, (v & 0xFFFF) as u16);
                        }
                        if decoded.is_some() {
                            out.push_str(decoded.as_deref().unwrap());
                            i += 3 + 5;
                            continue;
                        }
                    }
                }
                // 4-digit: `\M+XXXX` — code in the drawing's code page.
                if n >= 4 {
                    if let Ok(v) = u32::from_str_radix(&hex[..4], 16) {
                        decoded = decode_pair(doc_enc, (v & 0xFFFF) as u16);
                    }
                    if decoded.is_some() {
                        out.push_str(decoded.as_deref().unwrap());
                        i += 3 + 4;
                        continue;
                    }
                }
                // Not a valid escape — keep the backslash literally.
                out.push('\\');
                i += 1;
                continue;
            }
            if (kind == b'U' || kind == b'u') && i + 7 <= bytes.len() {
                // `\U+XXXX` — Unicode (CIF) code, exactly 4 hex digits.
                let hex = &text[i + 3..i + 7];
                if hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    if let Ok(v) = u16::from_str_radix(hex, 16) {
                        if let Some(ch) = char::from_u32(v as u32) {
                            out.push(ch);
                            i += 7;
                            continue;
                        }
                    }
                }
            }
            // Not a valid escape — keep the backslash literally.
            out.push('\\');
            i += 1;
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_gbk_four_digit() {
        // `\M+C4E3` → GBK C4 E3 = 你
        assert_eq!(decode(r"\M+C4E3", "ANSI_936"), "你");
    }

    #[test]
    fn decodes_index_five_gbk() {
        // ODA FAQ example: `\M+5BFAA` → index 5 (GBK), code BF AA = 开,
        // `\M+5B9D8` → B9 D8 = 关
        assert_eq!(decode(r"\M+5BFAA\M+5B9D8", "ANSI_936"), "开关");
    }

    #[test]
    fn decodes_index_one_shift_jis() {
        // From the user's real drawing (`1吨 地牛-川字托盘.DXF`): index 1
        // (Windows-31J), codes 8B 5A = 技, 8C 8F = 件, 92 8D = 注.
        assert_eq!(decode(r"\M+18B5A\M+18C8F\M+1928D", "ANSI_936"), "技件注");
        // 0xFC4B is only reachable through cp932 (Windows-31J): 黑
        assert_eq!(decode(r"\M+1FC4B", "ANSI_936"), "黑");
    }

    #[test]
    fn mif_is_fixed_width() {
        // A 5-digit MIF code never swallows following hex-looking text:
        // `\M+18A70` = 角, "C1" stays text.
        assert_eq!(decode(r"\M+18A70C1", "ANSI_936"), "角C1");
        // `\M+198B0` = 于, "0.5mm" stays text.
        assert_eq!(decode(r"\M+198B00.5mm", "ANSI_936"), "于0.5mm");
    }

    #[test]
    fn decodes_mixed_ascii_and_cjk() {
        // The user's original example `\M+19094\M+18146`:
        // index 1 (Windows-31J), 90 94 = 数, 81 46 = ：(full-width colon) —
        // the "数量" table header from the drawing.
        assert_eq!(decode(r"\M+19094\M+18146", "ANSI_936"), "数：");
    }

    #[test]
    fn leaves_invalid_escapes_literal() {
        // Leading byte below 0x80 → not a double-byte code → kept verbatim.
        assert_eq!(decode(r"\M+0041", "ANSI_936"), r"\M+0041");
        assert_eq!(decode(r"\M+12", "ANSI_936"), r"\M+12");
    }

    #[test]
    fn passes_through_plain_text() {
        assert_eq!(decode("Hello, 世界!", "ANSI_936"), "Hello, 世界!");
        assert_eq!(
            decode(r"\P is a paragraph break", "ANSI_936"),
            r"\P is a paragraph break"
        );
    }

    #[test]
    fn lower_case_m_escape() {
        assert_eq!(decode(r"\m+C4E3", "ANSI_936"), "你");
    }

    #[test]
    fn decodes_unicode_cif_escapes() {
        // `\U+4F60` = 你, `\U+597D` = 好, `\u+00E9` = é
        assert_eq!(decode(r"\U+4F60\U+597D", "ANSI_936"), "你好");
        assert_eq!(decode(r"\u+00E9", "ANSI_936"), "é");
    }

    #[test]
    fn leaves_invalid_unicode_escape_literal() {
        // A surrogate half is not a valid scalar value → kept verbatim.
        assert_eq!(decode(r"\U+D83D", "ANSI_936"), r"\U+D83D");
    }
}
