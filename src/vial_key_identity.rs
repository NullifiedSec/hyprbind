//! Translate Vial/QMK key labels into host-visible identity without pretending
//! firmware-level actions are ordinary Linux keys.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareKeyIdentity {
    HostKey(String),
    HostModifier(&'static str),
    Transparent,
    Disabled,
    LayerAction(String),
    MacroAction(String),
    TapDance(String),
    FirmwareAction(String),
}

impl HardwareKeyIdentity {
    pub fn host_key(&self) -> Option<&str> {
        match self {
            Self::HostKey(key) => Some(key),
            _ => None,
        }
    }

    pub fn host_modifier(&self) -> Option<&'static str> {
        match self {
            Self::HostModifier(modifier) => Some(*modifier),
            _ => None,
        }
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self, Self::Transparent)
    }

    pub fn explanation(&self) -> String {
        match self {
            Self::HostKey(key) => format!("emits host key {key}"),
            Self::HostModifier(modifier) => format!("emits host modifier {modifier}"),
            Self::Transparent => "transparent: inherits a lower hardware layer".into(),
            Self::Disabled => "disabled / no host event".into(),
            Self::LayerAction(label) => format!("firmware layer action: {label}"),
            Self::MacroAction(label) => format!("firmware macro action: {label}"),
            Self::TapDance(label) => format!("firmware tap-dance action: {label}"),
            Self::FirmwareAction(label) => format!("firmware-only/opaque action: {label}"),
        }
    }
}

pub fn classify_qmk_label(label: &str) -> HardwareKeyIdentity {
    let raw = label.trim();
    if raw.is_empty() {
        return HardwareKeyIdentity::Disabled;
    }
    let mut upper = raw.to_ascii_uppercase();
    if let Some(stripped) = upper.strip_prefix("KC_") {
        upper = stripped.to_string();
    }

    if matches!(upper.as_str(), "▽" | "TRNS" | "TRANSPARENT") {
        return HardwareKeyIdentity::Transparent;
    }
    if matches!(upper.as_str(), "NO" | "NONE") {
        return HardwareKeyIdentity::Disabled;
    }

    let modifier = match upper.as_str() {
        "LGUI" | "RGUI" | "LWIN" | "RWIN" | "LCMD" | "RCMD" => Some("SUPER"),
        "LCTL" | "RCTL" | "LCTRL" | "RCTRL" => Some("CTRL"),
        "LALT" | "RALT" | "LOPT" | "ROPT" => Some("ALT"),
        "LSFT" | "RSFT" | "LSHIFT" | "RSHIFT" => Some("SHIFT"),
        _ => None,
    };
    if let Some(modifier) = modifier {
        return HardwareKeyIdentity::HostModifier(modifier);
    }

    if starts_any(
        &upper,
        &["MO(", "TG(", "TO(", "DF(", "OSL(", "LM(", "LT(", "MT(", "TT("],
    ) {
        return HardwareKeyIdentity::LayerAction(raw.to_string());
    }
    if upper.starts_with("TD(") || upper.starts_with("TD_") {
        return HardwareKeyIdentity::TapDance(raw.to_string());
    }
    if upper.starts_with("MACRO")
        || upper.starts_with("M(")
        || upper.strip_prefix('M').is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
    {
        return HardwareKeyIdentity::MacroAction(raw.to_string());
    }
    if starts_any(
        &upper,
        &[
            "QK_", "USER", "SAFE_RANGE", "COMBO", "ALTREP", "REPEAT", "VIAL", "RGB_", "BL_",
        ],
    ) {
        return HardwareKeyIdentity::FirmwareAction(raw.to_string());
    }

    if upper.len() == 1 {
        let c = upper.chars().next().unwrap();
        if c.is_ascii_alphanumeric() {
            return HardwareKeyIdentity::HostKey(upper);
        }
    }
    if let Some(rest) = upper.strip_prefix('F') {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return HardwareKeyIdentity::HostKey(upper);
        }
    }

    let mapped = match upper.as_str() {
        "ESC" | "ESCAPE" => "Escape",
        "ENT" | "ENTER" | "RETURN" => "Return",
        "BSPC" | "BKSP" | "BACKSPACE" => "BackSpace",
        "TAB" => "Tab",
        "SPC" | "SPACE" => "SPACE",
        "DEL" | "DELETE" => "Delete",
        "INS" | "INSERT" => "Insert",
        "HOME" => "Home",
        "END" => "End",
        "PGUP" | "PAGEUP" => "Page_Up",
        "PGDN" | "PAGEDOWN" => "Page_Down",
        "LEFT" => "left",
        "RIGHT" => "right",
        "UP" => "up",
        "DOWN" => "down",
        "CAPS" | "CAPSLOCK" | "CAPS_LOCK" => "Caps_Lock",
        "PSCR" | "PRINTSCREEN" => "Print",
        "SLCK" | "SCROLLLOCK" => "Scroll_Lock",
        "PAUS" | "PAUSE" => "Pause",
        "NUM" | "NUMLOCK" => "Num_Lock",
        "MINS" | "MINUS" => "minus",
        "EQL" | "EQUAL" => "equal",
        "LBRC" | "LBRACKET" => "bracketleft",
        "RBRC" | "RBRACKET" => "bracketright",
        "BSLS" | "BACKSLASH" => "backslash",
        "NUHS" => "numbersign",
        "SCLN" | "SEMICOLON" => "semicolon",
        "QUOT" | "QUOTE" => "apostrophe",
        "GRV" | "GRAVE" => "grave",
        "COMM" | "COMMA" => "comma",
        "DOT" | "PERIOD" => "period",
        "SLSH" | "SLASH" => "slash",
        "MUTE" | "AUDIO_MUTE" => "XF86AudioMute",
        "VOLU" | "VOL+" | "AUDIO_VOL_UP" => "XF86AudioRaiseVolume",
        "VOLD" | "VOL-" | "AUDIO_VOL_DOWN" => "XF86AudioLowerVolume",
        "MPLY" | "PLAY" | "MEDIA_PLAY_PAUSE" => "XF86AudioPlay",
        "MNXT" | "NEXT" | "MEDIA_NEXT_TRACK" => "XF86AudioNext",
        "MPRV" | "PREV" | "MEDIA_PREV_TRACK" => "XF86AudioPrev",
        "MSTP" | "MEDIA_STOP" => "XF86AudioStop",
        "BRIU" | "BRIGHTNESS_UP" => "XF86MonBrightnessUp",
        "BRID" | "BRIGHTNESS_DOWN" => "XF86MonBrightnessDown",
        _ => return HardwareKeyIdentity::FirmwareAction(raw.to_string()),
    };
    HardwareKeyIdentity::HostKey(mapped.to_string())
}

fn starts_any(value: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| value.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_fake_layer_tap_as_host_key() {
        assert!(matches!(
            classify_qmk_label("LT(2, KC_A)"),
            HardwareKeyIdentity::LayerAction(_)
        ));
    }

    #[test]
    fn maps_simple_qmk_labels_to_xkb_names() {
        assert_eq!(classify_qmk_label("KC_ESC").host_key(), Some("Escape"));
        assert_eq!(classify_qmk_label("MINS").host_key(), Some("minus"));
        assert_eq!(classify_qmk_label("A").host_key(), Some("A"));
    }

    #[test]
    fn modifier_identity_is_explicit() {
        assert_eq!(classify_qmk_label("LGUI").host_modifier(), Some("SUPER"));
        assert_eq!(classify_qmk_label("RCTL").host_modifier(), Some("CTRL"));
    }
}
