use gtk4::gdk::{Key, ModifierType};

/// Convert a GTK key event into a Hyprland bind key string, e.g. `SUPER + SHIFT + Q`.
pub fn event_to_hypr_keys(key: Key, modifiers: ModifierType) -> Option<String> {
    if is_modifier_key(key) {
        return None;
    }

    let key_name = key_to_hypr_name(key)?;
    let mut parts = Vec::new();

    if modifiers.contains(ModifierType::SUPER_MASK) || modifiers.contains(ModifierType::META_MASK) {
        parts.push("SUPER");
    }
    if modifiers.contains(ModifierType::CONTROL_MASK) {
        parts.push("CTRL");
    }
    if modifiers.contains(ModifierType::ALT_MASK) {
        parts.push("ALT");
    }
    if modifiers.contains(ModifierType::SHIFT_MASK) {
        parts.push("SHIFT");
    }

    parts.push(key_name.as_str());
    Some(parts.join(" + "))
}

pub fn normalize_keys_input(raw: &str) -> String {
    raw.split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            // Preserve {{variable}} tokens as-is.
            if part.starts_with("{{") && part.ends_with("}}") {
                return part.to_string();
            }
            if part.len() == 1 {
                return part.to_uppercase();
            }
            match part.to_ascii_uppercase().as_str() {
                "SUPER" | "SHIFT" | "CTRL" | "ALT" | "SPACE" => part.to_ascii_uppercase(),
                "CONTROL" => "CTRL".to_string(),
                "META" => "SUPER".to_string(),
                _ => part.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn is_modifier_key(key: Key) -> bool {
    matches!(
        key,
        Key::Shift_L
            | Key::Shift_R
            | Key::Control_L
            | Key::Control_R
            | Key::Alt_L
            | Key::Alt_R
            | Key::Meta_L
            | Key::Meta_R
            | Key::Super_L
            | Key::Super_R
            | Key::Hyper_L
            | Key::Hyper_R
            | Key::ISO_Level3_Shift
            | Key::Mode_switch
    )
}

fn key_to_hypr_name(key: Key) -> Option<String> {
    let special = match key {
        Key::Return | Key::KP_Enter => Some("Return"),
        Key::Escape => Some("Escape"),
        Key::BackSpace => Some("BackSpace"),
        Key::Tab => Some("Tab"),
        Key::space => Some("SPACE"),
        Key::Delete => Some("DELETE"),
        Key::Insert => Some("Insert"),
        Key::Home => Some("Home"),
        Key::End => Some("End"),
        Key::Page_Up => Some("Page_Up"),
        Key::Page_Down => Some("Page_Down"),
        Key::Left => Some("left"),
        Key::Right => Some("right"),
        Key::Up => Some("up"),
        Key::Down => Some("down"),
        Key::Print => Some("Print"),
        Key::Pause => Some("Pause"),
        Key::Menu => Some("Menu"),
        Key::F1 => Some("F1"),
        Key::F2 => Some("F2"),
        Key::F3 => Some("F3"),
        Key::F4 => Some("F4"),
        Key::F5 => Some("F5"),
        Key::F6 => Some("F6"),
        Key::F7 => Some("F7"),
        Key::F8 => Some("F8"),
        Key::F9 => Some("F9"),
        Key::F10 => Some("F10"),
        Key::F11 => Some("F11"),
        Key::F12 => Some("F12"),
        Key::AudioRaiseVolume => Some("XF86AudioRaiseVolume"),
        Key::AudioLowerVolume => Some("XF86AudioLowerVolume"),
        Key::AudioMute => Some("XF86AudioMute"),
        Key::AudioPlay => Some("XF86AudioPlay"),
        Key::AudioPause => Some("XF86AudioPause"),
        Key::AudioNext => Some("XF86AudioNext"),
        Key::AudioPrev => Some("XF86AudioPrev"),
        Key::MonBrightnessUp => Some("XF86MonBrightnessUp"),
        Key::MonBrightnessDown => Some("XF86MonBrightnessDown"),
        _ => None,
    };
    if let Some(name) = special {
        return Some(name.to_string());
    }

    let name = key.name()?.as_str().to_string();
    if name.len() == 1 {
        return Some(name.to_uppercase());
    }
    if name.chars().all(|c| c.is_ascii_digit()) {
        return Some(name);
    }
    Some(name)
}

pub fn lua_string_literal(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}
