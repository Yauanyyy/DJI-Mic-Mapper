#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: Vec<u16>,
    pub key: u16,
    pub display: String,
}

impl Chord {
    pub fn parse(input: &str) -> Result<Self, String> {
        let parts: Vec<_> = input
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return Err("target cannot be empty".to_owned());
        }

        let mut modifiers = Vec::new();
        for part in &parts[..parts.len() - 1] {
            let code = match normalize(part).as_str() {
                "CTRL" | "CONTROL" => 0x11,
                "ALT" => 0x12,
                "SHIFT" => 0x10,
                "WIN" | "WINDOWS" | "META" => 0x5B,
                _ => return Err(format!("unknown modifier '{part}'")),
            };
            if modifiers.contains(&code) {
                return Err(format!("duplicate modifier '{part}'"));
            }
            modifiers.push(code);
        }

        let main = parts[parts.len() - 1];
        let key = parse_main_key(main).ok_or_else(|| format!("unknown target key '{main}'"))?;
        if matches!(key, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C) {
            return Err("a modifier cannot be the main target key".to_owned());
        }

        Ok(Self {
            modifiers,
            key,
            display: parts.join("+"),
        })
    }
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .replace([' ', '-', '_'], "")
        .to_ascii_uppercase()
}

fn parse_main_key(value: &str) -> Option<u16> {
    let key = normalize(value);
    if key.len() == 1 {
        let byte = key.as_bytes()[0];
        if byte.is_ascii_uppercase() || byte.is_ascii_digit() {
            return Some(byte as u16);
        }
    }
    if let Some(number) = key.strip_prefix('F').and_then(|n| n.parse::<u16>().ok())
        && (1..=24).contains(&number)
    {
        return Some(0x70 + number - 1);
    }
    if let Some(number) = key
        .strip_prefix("NUMPAD")
        .and_then(|n| n.parse::<u16>().ok())
        && number <= 9
    {
        return Some(0x60 + number);
    }

    Some(match key.as_str() {
        "BACKSPACE" | "BACK" => 0x08,
        "TAB" => 0x09,
        "ENTER" | "RETURN" => 0x0D,
        "ESC" | "ESCAPE" => 0x1B,
        "SPACE" => 0x20,
        "PAGEUP" | "PGUP" => 0x21,
        "PAGEDOWN" | "PGDN" => 0x22,
        "END" => 0x23,
        "HOME" => 0x24,
        "LEFT" => 0x25,
        "UP" => 0x26,
        "RIGHT" => 0x27,
        "DOWN" => 0x28,
        "INSERT" | "INS" => 0x2D,
        "DELETE" | "DEL" => 0x2E,
        "MULTIPLY" => 0x6A,
        "ADD" => 0x6B,
        "SUBTRACT" => 0x6D,
        "DECIMAL" => 0x6E,
        "DIVIDE" => 0x6F,
        "NUMLOCK" => 0x90,
        "SCROLLLOCK" => 0x91,
        "VOLUMEMUTE" | "MUTE" => 0xAD,
        "VOLUMEDOWN" => 0xAE,
        "VOLUMEUP" => 0xAF,
        "NEXTTRACK" => 0xB0,
        "PREVTRACK" | "PREVIOUSTRACK" => 0xB1,
        "STOPMEDIA" | "MEDIASTOP" => 0xB2,
        "PLAYPAUSE" | "MEDIAPLAYPAUSE" => 0xB3,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_f13() {
        assert_eq!(Chord::parse("F13").unwrap().key, 0x7C);
    }

    #[test]
    fn parses_a_chord_case_insensitively() {
        let chord = Chord::parse("ctrl + Shift + f13").unwrap();
        assert_eq!(chord.modifiers, vec![0x11, 0x10]);
        assert_eq!(chord.key, 0x7C);
    }

    #[test]
    fn rejects_duplicate_modifiers() {
        assert!(Chord::parse("Ctrl+Control+F13").is_err());
    }

    #[test]
    fn parses_media_key_names() {
        assert_eq!(Chord::parse("Volume_Up").unwrap().key, 0xAF);
        assert_eq!(Chord::parse("Play-Pause").unwrap().key, 0xB3);
    }
}
