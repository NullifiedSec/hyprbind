//! Waybar module catalog and editable field schemas.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    String,
    Int,
    Bool,
    Json,
}

#[derive(Debug, Clone)]
pub struct ModuleField {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub hint: &'static str,
}

#[derive(Debug, Clone)]
pub struct ModuleSchema {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub fields: &'static [ModuleField],
}

const COMMON_FORMAT: ModuleField = ModuleField {
    key: "format",
    label: "Format",
    kind: FieldKind::String,
    hint: "Display format string",
};

const COMMON_TOOLTIP: ModuleField = ModuleField {
    key: "tooltip",
    label: "Tooltip",
    kind: FieldKind::Bool,
    hint: "Show tooltip",
};

const COMMON_INTERVAL: ModuleField = ModuleField {
    key: "interval",
    label: "Interval",
    kind: FieldKind::Int,
    hint: "Update interval (seconds)",
};

const COMMON_ON_CLICK: ModuleField = ModuleField {
    key: "on-click",
    label: "On click",
    kind: FieldKind::String,
    hint: "Command on left click",
};

const COMMON_MAX_LENGTH: ModuleField = ModuleField {
    key: "max-length",
    label: "Max length",
    kind: FieldKind::Int,
    hint: "Truncate text length",
};

pub fn catalog() -> Vec<ModuleSchema> {
    vec![
        ModuleSchema {
            id: "hyprland/workspaces",
            label: "Hyprland Workspaces",
            category: "Hyprland",
            fields: &[
                COMMON_FORMAT,
                ModuleField {
                    key: "all-outputs",
                    label: "All outputs",
                    kind: FieldKind::Bool,
                    hint: "Show workspaces on all monitors",
                },
                ModuleField {
                    key: "disable-scroll",
                    label: "Disable scroll",
                    kind: FieldKind::Bool,
                    hint: "Disable scroll workspace switching",
                },
                ModuleField {
                    key: "on-click",
                    label: "On click",
                    kind: FieldKind::String,
                    hint: "Usually \"activate\"",
                },
                ModuleField {
                    key: "sort-by",
                    label: "Sort by",
                    kind: FieldKind::String,
                    hint: "number | name | id | default",
                },
                // format-icons + persistent-workspaces use a dedicated UI panel
            ],
        },
        ModuleSchema {
            id: "hyprland/window",
            label: "Hyprland Window",
            category: "Hyprland",
            fields: &[COMMON_FORMAT, COMMON_MAX_LENGTH, COMMON_TOOLTIP],
        },
        ModuleSchema {
            id: "hyprland/language",
            label: "Hyprland Language",
            category: "Hyprland",
            fields: &[COMMON_FORMAT, COMMON_TOOLTIP, COMMON_ON_CLICK],
        },
        ModuleSchema {
            id: "clock",
            label: "Clock",
            category: "System",
            fields: &[
                COMMON_FORMAT,
                ModuleField {
                    key: "format-alt",
                    label: "Alt format",
                    kind: FieldKind::String,
                    hint: "Alternate format on click",
                },
                ModuleField {
                    key: "tooltip-format",
                    label: "Tooltip format",
                    kind: FieldKind::String,
                    hint: "Tooltip content",
                },
                COMMON_INTERVAL,
            ],
        },
        ModuleSchema {
            id: "cpu",
            label: "CPU",
            category: "System",
            fields: &[COMMON_FORMAT, COMMON_INTERVAL, COMMON_TOOLTIP],
        },
        ModuleSchema {
            id: "memory",
            label: "Memory",
            category: "System",
            fields: &[COMMON_FORMAT, COMMON_INTERVAL, COMMON_TOOLTIP],
        },
        ModuleSchema {
            id: "battery",
            label: "Battery",
            category: "System",
            fields: &[
                COMMON_FORMAT,
                ModuleField {
                    key: "format-charging",
                    label: "Charging format",
                    kind: FieldKind::String,
                    hint: "When charging",
                },
                ModuleField {
                    key: "format-plugged",
                    label: "Plugged format",
                    kind: FieldKind::String,
                    hint: "When plugged",
                },
                ModuleField {
                    key: "format-icons",
                    label: "Icons",
                    kind: FieldKind::Json,
                    hint: "Icon list or map",
                },
                ModuleField {
                    key: "states",
                    label: "States",
                    kind: FieldKind::Json,
                    hint: "warning/critical thresholds",
                },
            ],
        },
        ModuleSchema {
            id: "pulseaudio",
            label: "PulseAudio",
            category: "System",
            fields: &[
                COMMON_FORMAT,
                ModuleField {
                    key: "format-muted",
                    label: "Muted format",
                    kind: FieldKind::String,
                    hint: "When muted",
                },
                ModuleField {
                    key: "format-icons",
                    label: "Icons",
                    kind: FieldKind::Json,
                    hint: "Device icon map",
                },
                COMMON_ON_CLICK,
            ],
        },
        ModuleSchema {
            id: "bluetooth",
            label: "Bluetooth",
            category: "System",
            fields: &[
                COMMON_FORMAT,
                ModuleField {
                    key: "format-disabled",
                    label: "Disabled format",
                    kind: FieldKind::String,
                    hint: "When disabled",
                },
                ModuleField {
                    key: "format-connected",
                    label: "Connected format",
                    kind: FieldKind::String,
                    hint: "When connected",
                },
                COMMON_ON_CLICK,
                COMMON_MAX_LENGTH,
            ],
        },
        ModuleSchema {
            id: "network",
            label: "Network",
            category: "System",
            fields: &[
                COMMON_FORMAT,
                ModuleField {
                    key: "format-wifi",
                    label: "Wi-Fi format",
                    kind: FieldKind::String,
                    hint: "Wi-Fi connected",
                },
                ModuleField {
                    key: "format-ethernet",
                    label: "Ethernet format",
                    kind: FieldKind::String,
                    hint: "Ethernet connected",
                },
                ModuleField {
                    key: "format-disconnected",
                    label: "Disconnected",
                    kind: FieldKind::String,
                    hint: "When offline",
                },
                COMMON_ON_CLICK,
                COMMON_TOOLTIP,
            ],
        },
        ModuleSchema {
            id: "tray",
            label: "System Tray",
            category: "System",
            fields: &[
                ModuleField {
                    key: "icon-size",
                    label: "Icon size",
                    kind: FieldKind::Int,
                    hint: "Tray icon size",
                },
                ModuleField {
                    key: "spacing",
                    label: "Spacing",
                    kind: FieldKind::Int,
                    hint: "Space between icons",
                },
            ],
        },
    ]
}

pub fn schema_for(module_id: &str) -> Option<ModuleSchema> {
    if let Some(s) = catalog().into_iter().find(|s| s.id == module_id) {
        return Some(s);
    }
    if module_id.starts_with("custom/") {
        return Some(custom_schema(module_id));
    }
    if module_id.starts_with("group/") {
        return Some(group_schema(module_id));
    }
    None
}

pub fn custom_schema(id: &str) -> ModuleSchema {
    // Leak id for 'static — only used for display; use a static placeholder.
    let _ = id;
    ModuleSchema {
        id: "custom/*",
        label: "Custom module",
        category: "Custom",
        fields: &[
            COMMON_FORMAT,
            ModuleField {
                key: "exec",
                label: "Exec",
                kind: FieldKind::String,
                hint: "Command to run",
            },
            ModuleField {
                key: "return-type",
                label: "Return type",
                kind: FieldKind::String,
                hint: "json or empty",
            },
            ModuleField {
                key: "signal",
                label: "Signal",
                kind: FieldKind::Int,
                hint: "RT signal number",
            },
            COMMON_INTERVAL,
            COMMON_ON_CLICK,
            ModuleField {
                key: "on-click-right",
                label: "Right click",
                kind: FieldKind::String,
                hint: "Right-click command",
            },
            ModuleField {
                key: "on-click-middle",
                label: "Middle click",
                kind: FieldKind::String,
                hint: "Middle-click command",
            },
            ModuleField {
                key: "on-scroll-up",
                label: "Scroll up",
                kind: FieldKind::String,
                hint: "Scroll-up command",
            },
            ModuleField {
                key: "on-scroll-down",
                label: "Scroll down",
                kind: FieldKind::String,
                hint: "Scroll-down command",
            },
            COMMON_TOOLTIP,
        ],
    }
}

pub fn group_schema(id: &str) -> ModuleSchema {
    let _ = id;
    ModuleSchema {
        id: "group/*",
        label: "Group",
        category: "Group",
        fields: &[
            ModuleField {
                key: "orientation",
                label: "Orientation",
                kind: FieldKind::String,
                hint: "horizontal / vertical / inherit",
            },
            ModuleField {
                key: "modules",
                label: "Child modules",
                kind: FieldKind::Json,
                hint: "JSON array of module ids",
            },
            ModuleField {
                key: "drawer",
                label: "Drawer",
                kind: FieldKind::Json,
                hint: "Drawer options object",
            },
        ],
    }
}

pub fn addable_modules() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = catalog().into_iter().map(|s| s.id).collect();
    ids.push("custom/example");
    ids
}

pub fn default_config_for(name: &str) -> Value {
    if name.starts_with("custom/") {
        return json!({
            "format": "{}",
            "exec": "echo hello",
            "interval": 5
        });
    }
    if name.starts_with("group/") {
        return json!({
            "orientation": "inherit",
            "modules": []
        });
    }
    match name {
        "hyprland/workspaces" => json!({
            "disable-scroll": false,
            "all-outputs": true,
            "format": "{icon}",
            "on-click": "activate",
            "sort-by": "number",
            "format-icons": {
                "1": "1",
                "2": "2",
                "3": "3",
                "4": "4",
                "5": "5",
                "6": "6",
                "7": "7",
                "8": "8",
                "9": "9",
                "10": "10",
                "urgent": "!",
                "active": "●",
                "default": "○",
                "empty": "○"
            },
            "persistent-workspaces": { "*": 5 }
        }),
        "hyprland/window" => json!({
            "format": "{title}",
            "max-length": 40
        }),
        "hyprland/language" => json!({
            "format": "{short}"
        }),
        "clock" => json!({
            "format": "{:%H:%M}"
        }),
        "cpu" => json!({
            "format": "CPU {usage}%",
            "interval": 2
        }),
        "memory" => json!({
            "format": "MEM {}%",
            "interval": 2
        }),
        "battery" => json!({
            "format": "{capacity}%",
            "format-charging": "⚡ {capacity}%"
        }),
        "pulseaudio" => json!({
            "format": "{icon} {volume}%",
            "format-muted": "muted",
            "on-click": "pavucontrol"
        }),
        "bluetooth" => json!({
            "format": "BT",
            "on-click": "blueman-manager"
        }),
        "network" => json!({
            "format-wifi": "{essid}",
            "format-ethernet": "ETH",
            "format-disconnected": "offline"
        }),
        "tray" => json!({
            "icon-size": 16,
            "spacing": 10
        }),
        _ => json!({}),
    }
}
