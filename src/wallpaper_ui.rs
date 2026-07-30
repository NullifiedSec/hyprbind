//! Wallpaper page — awww daemon control, image picker, and recent wallpapers.

use crate::dialog;
use crate::wallpaper::{self, OutputWallpaper};
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, StringList, Window,
};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct WallpaperPage {
    pub page: GtkBox,
}

const TRANSITIONS: &[&str] = &[
    "fade", "simple", "left", "right", "top", "bottom", "wipe", "grow", "center", "outer", "any",
    "random",
];
const RESIZE_MODES: &[&str] = &["crop", "fit", "stretch", "no"];

fn dropdown_label(dd: &DropDown, labels: &[&str]) -> String {
    let idx = dd.selected() as usize;
    labels.get(idx).map(|s| (*s).to_string()).unwrap_or_default()
}

fn selected_output(dd: &DropDown) -> Option<String> {
    let idx = dd.selected() as usize;
    if idx == 0 {
        return None;
    }
    dd.model()
        .and_then(|m| m.item(idx as u32))
        .and_then(|item| item.downcast::<gtk4::StringObject>().ok())
        .map(|o| o.string().to_string())
}

fn fill_output_dropdown(dd: &DropDown, outputs: &[OutputWallpaper]) {
    let mut labels = vec!["All outputs".to_string()];
    labels.extend(outputs.iter().map(|o| o.output.clone()));
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    dd.set_model(Some(&StringList::new(&refs)));
    dd.set_selected(0);
}

pub fn build_wallpaper_page(
    parent: &impl IsA<Window>,
    status: Rc<dyn Fn(String)>,
) -> WallpaperPage {
    let ensure_btn = Button::builder()
        .label("Ensure daemon")
        .css_classes(["suggested-action"])
        .build();
    let refresh_btn = Button::builder().label("Query / Refresh").build();
    let restore_btn = Button::builder().label("Restore").build();
    let clear_btn = Button::builder().label("Clear").build();
    let pick_btn = Button::builder().label("Pick image…").build();

    let color_entry = Entry::builder()
        .text("111111")
        .width_chars(8)
        .build();

    let transition_dd = DropDown::from_strings(TRANSITIONS);
    let resize_dd = DropDown::from_strings(RESIZE_MODES);
    let output_dd = DropDown::from_strings(&["All outputs"]);

    let prefs = wallpaper::load_prefs();
    if let Some(idx) = TRANSITIONS
        .iter()
        .position(|t| *t == prefs.transition)
    {
        transition_dd.set_selected(idx as u32);
    }
    if let Some(idx) = RESIZE_MODES.iter().position(|t| *t == prefs.resize) {
        resize_dd.set_selected(idx as u32);
    }

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.append(&ensure_btn);
    actions.append(&refresh_btn);
    actions.append(&restore_btn);
    actions.append(&clear_btn);
    actions.append(&color_entry);
    actions.append(&pick_btn);

    let options = GtkBox::new(Orientation::Horizontal, 12);
    options.append(&option_block("Transition", &transition_dd));
    options.append(&option_block("Resize", &resize_dd));
    options.append(&option_block("Output", &output_dd));

    let toolbar = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    toolbar.append(&actions);
    toolbar.append(&options);

    let outputs_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let recent_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(4)
        .margin_bottom(12)
        .build();
    content.append(&dialog::section_label("Outputs"));
    content.append(&outputs_list);
    content.append(&dialog::section_label("Recent"));
    content.append(&recent_list);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&content)
        .build();

    let page = dialog::page_shell(
        "Wallpaper",
        "Manage wallpapers with awww. Ensure the daemon is running, pick an image, or re-apply a recent path.",
        &toolbar,
        &scroll,
    );

    let refresh = {
        let outputs_list = outputs_list.clone();
        let recent_list = recent_list.clone();
        let output_dd = output_dd.clone();
        let status = Rc::clone(&status);
        Rc::new(move || {
            match wallpaper::query() {
                Ok(outputs) => {
                    while let Some(child) = outputs_list.first_child() {
                        outputs_list.remove(&child);
                    }
                    if outputs.is_empty() {
                        outputs_list.append(&empty_row("No outputs from awww query"));
                    } else {
                        for row in &outputs {
                            outputs_list.append(&output_row(row));
                        }
                    }
                    fill_output_dropdown(&output_dd, &outputs);
                    status(format!("Queried {} output(s)", outputs.len()));
                }
                Err(e) => status(format!("awww query failed: {e}")),
            }

            let prefs = wallpaper::load_prefs();
            while let Some(child) = recent_list.first_child() {
                recent_list.remove(&child);
            }
            if prefs.recent.is_empty() {
                recent_list.append(&empty_row("No recent wallpapers"));
            } else {
                for path in &prefs.recent {
                    recent_list.append(&recent_row(path));
                }
            }
        })
    };

    let apply_wallpaper = {
        let transition_dd = transition_dd.clone();
        let resize_dd = resize_dd.clone();
        let output_dd = output_dd.clone();
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        Rc::new(move |path: PathBuf| {
            let transition = dropdown_label(&transition_dd, TRANSITIONS);
            let resize = dropdown_label(&resize_dd, RESIZE_MODES);
            let output_owned = selected_output(&output_dd);
            let output_opt = output_owned.as_deref();
            match wallpaper::set_image(&path, &transition, &resize, output_opt) {
                Ok(msg) => {
                    status(msg);
                    refresh();
                }
                Err(e) => status(format!("Set wallpaper failed: {e}")),
            }
        })
    };

    ensure_btn.connect_clicked({
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| match wallpaper::ensure_daemon() {
            Ok(msg) => {
                status(msg);
                refresh();
            }
            Err(e) => status(e),
        }
    });

    refresh_btn.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });

    restore_btn.connect_clicked({
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| match wallpaper::restore() {
            Ok(msg) => {
                status(msg);
                refresh();
            }
            Err(e) => status(format!("Restore failed: {e}")),
        }
    });

    clear_btn.connect_clicked({
        let color_entry = color_entry.clone();
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| {
            let color = color_entry.text().to_string();
            match wallpaper::clear(&color) {
                Ok(msg) => {
                    status(msg);
                    refresh();
                }
                Err(e) => status(format!("Clear failed: {e}")),
            }
        }
    });

    pick_btn.connect_clicked({
        let parent = parent.clone().upcast::<Window>();
        let apply_wallpaper = Rc::clone(&apply_wallpaper);
        move |_| {
            let apply_wallpaper = Rc::clone(&apply_wallpaper);
            let dialog = gtk4::FileDialog::builder().title("Choose wallpaper").build();
            let filter = gtk4::FileFilter::new();
            filter.add_mime_type("image/*");
            filter.set_name(Some("Images"));
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            dialog.open(Some(&parent), None::<&gio::Cancellable>, move |result| {
                let Ok(file) = result else {
                    return;
                };
                let Some(path) = file.path() else {
                    return;
                };
                apply_wallpaper(path);
            });
        }
    });

    recent_list.connect_row_activated({
        let apply_wallpaper = Rc::clone(&apply_wallpaper);
        move |_, row| {
            let path = row.widget_name().to_string();
            if !path.is_empty() && Path::new(&path).is_file() {
                apply_wallpaper(PathBuf::from(path));
            }
        }
    });

    match wallpaper::ensure_daemon() {
        Ok(msg) => status(msg),
        Err(e) => status(e),
    }
    refresh();

    WallpaperPage { page }
}

fn option_block(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .hexpand(true)
        .build();
    block.append(
        &Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    block.append(widget);
    block
}

fn empty_row(text: &str) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_body(text));
    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}

fn output_row(row: &OutputWallpaper) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&row.output));
    col.append(&dialog::row_sub(&row.size));
    if !row.image.is_empty() {
        col.append(&dialog::row_body(&row.image));
    }
    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}

fn recent_row(path: &str) -> ListBoxRow {
    let col = dialog::list_row_column();
    let display = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    col.append(&dialog::row_title(display));
    col.append(&dialog::row_sub(path));
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(path)
        .build()
}
