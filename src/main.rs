mod backup;
mod bind;
mod bundle;
mod bundle_ui;
mod clients;
mod config;
mod conflicts;
mod curve_editor;
mod debounce;
mod dialog;
mod dispatcher_ui;
mod dispatchers;
mod env;
mod env_ui;
mod experimental;
mod extra_ui;
mod health;
mod health_ui;
mod keys;
mod logs;
mod logs_ui;
mod lookfeel_ui;
mod nav;
mod overview_ui;
mod palette;
mod qml_bridge;
mod qml_health_bridge;
mod qml_logs_bridge;
mod qml_startup_bridge;
mod qml_submaps_bridge;
mod settings_config;
mod spec;
mod startup;
mod startup_ui;
mod sysinfo;
mod ui;
mod ui_prefs;
mod variables;
mod window_rules;
mod writer;

use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "dev.hyprbinds.Hyprbinds";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--health") {
        let results = health::run_all();
        let (ok, warn, fail) = health::summarize(&results);
        println!("hyprbinds health: {ok} ok · {warn} warnings · {fail} failures\n");
        let mut last = "";
        for r in &results {
            if r.category != last {
                last = r.category;
                println!("## {}", r.category);
            }
            println!("[{}] {}", r.severity.label(), r.title);
            println!("  {}", r.detail);
            if let Some(h) = &r.fix_hint {
                println!("  hint: {h}");
            }
            if let Some(c) = &r.fix_command {
                println!("  fix: {c}");
            }
            println!();
        }
        std::process::exit(if fail > 0 { 1 } else { 0 });
    }
    if args.iter().any(|a| a == "--sysinfo") {
        print!("{}", sysinfo::collect_report());
        return;
    }
    if args.iter().any(|a| a == "--dump" || a == "--json") {
        match config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                if let Some(err) = &collection.error {
                    eprintln!("warning: {err}");
                }
                println!(
                    "config: {}\nbinds: {}\nvariables: {}\nenv: {}\nsubmaps: {}\nstartup: {}\nwindow rules: {}\nworkspace rules: {}\nlayer rules: {}\nmonitors: {}\ndevices: {}\ngestures: {}\ncurves: {}\nanimations: {}\nconfig sections: {}",
                    collection.config_path.display(),
                    collection.binds.len(),
                    collection.variables.len(),
                    collection.env.len(),
                    collection.submaps.len(),
                    collection.startup.len(),
                    collection.window_rules.len(),
                    collection.workspace_rules.len(),
                    collection.layer_rules.len(),
                    collection.monitors.len(),
                    collection.devices.len(),
                    collection.gestures.len(),
                    collection.curves.len(),
                    collection.animations.len(),
                    collection
                        .config_merged
                        .as_object()
                        .map(|o| o.len())
                        .unwrap_or(0)
                );
                if !collection.env.is_empty() {
                    println!("env:");
                    for e in &collection.env {
                        println!("  {} = {:?}", e.name, e.value);
                    }
                }
                if !collection.submaps.is_empty() {
                    println!("submaps:");
                    for s in &collection.submaps {
                        println!("  {} ({} binds)", s.name, s.bind_count);
                    }
                }
                if !collection.variables.is_empty() {
                    println!("vars:");
                    for var in &collection.variables {
                        println!("  {} = {:?}", var.name, var.value);
                    }
                }
                for bind in &collection.binds {
                    let flags = if bind.flags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", bind.flags_label())
                    };
                    let submap = if bind.submap.is_empty() {
                        String::new()
                    } else {
                        format!(" {{{}}}", bind.submap)
                    };
                    println!(
                        "{:<16} {:<28} -> {}{}{}  ({})",
                        bind.name,
                        bind.keys,
                        bind.action,
                        flags,
                        submap,
                        bind.source_label()
                    );
                }
            }
            Err(err) => {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    if let Some(idx) = args.iter().position(|a| a == "--export") {
        let path = args
            .get(idx + 1)
            .cloned()
            .unwrap_or_else(|| "hyprbinds-export.json".into());
        match config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                match bundle::export_to_path(&collection, std::path::Path::new(&path)) {
                    Ok(()) => {
                        println!("exported → {path}");
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Err(err) => {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        return;
    }
    if let Some(idx) = args.iter().position(|a| a == "--import") {
        let Some(path) = args.get(idx + 1) else {
            eprintln!("usage: hyprbinds --import <path.json>");
            std::process::exit(2);
        };
        match bundle::import_from_path(std::path::Path::new(path), bundle::ImportOptions::default())
        {
            Ok(report) => {
                println!("{}", report.summary());
                if !report.ok() {
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--qml") {
        qml_bridge::run();
        return;
    }

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(ui::build_ui);
    app.run();
}
