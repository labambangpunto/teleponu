use std::path::Path;
use std::rc::Rc;

use adw::prelude::*;
use gtk::gio;

pub fn show_preferences_dialog(
    parent: &impl IsA<gtk::Widget>,
    settings: &gio::Settings,
    current_open_path: Option<String>,
    on_state_changed: Rc<dyn Fn()>,
) {
    let dialog = adw::PreferencesDialog::builder()
        .title("Preferences")
        .build();

    let page = adw::PreferencesPage::new();

    // --- Group 1: Startup & Recent File Behavior ---
    let behavior_group = adw::PreferencesGroup::builder()
        .title("File History &amp; Startup")
        .description("Configure how Teleponu remembers and reopens VCF 3.0 files.")
        .build();

    let remember_row = adw::SwitchRow::builder()
        .title("Remember Recent File")
        .subtitle("Keep track of the most recently opened or saved file path")
        .build();

    let auto_open_row = adw::SwitchRow::builder()
        .title("Open Last File on Startup")
        .subtitle("Automatically load the last opened file when Teleponu launches")
        .build();

    settings
        .bind("remember-recent-file", &remember_row, "active")
        .build();
    settings
        .bind("auto-open-last-file", &auto_open_row, "active")
        .build();

    auto_open_row.set_sensitive(remember_row.is_active());

    behavior_group.add(&remember_row);
    behavior_group.add(&auto_open_row);
    page.add(&behavior_group);

    // --- Group 2: Stored Path Info & Clear Button ---
    let storage_group = adw::PreferencesGroup::builder()
        .title("Stored Recent Path")
        .build();

    let path_row = adw::ActionRow::builder()
        .title("Last File")
        .subtitle(&format_stored_path(settings))
        .use_markup(false)
        .build();

    let clear_btn = gtk::Button::builder()
        .label("Clear")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .sensitive(!settings.string("last-opened-file").is_empty())
        .build();

    path_row.add_suffix(&clear_btn);
    storage_group.add(&path_row);
    page.add(&storage_group);

    // Sync behavior when "Remember Recent File" is toggled
    {
        let settings = settings.clone();
        let auto_open_row_weak = auto_open_row.downgrade();
        let path_row_weak = path_row.downgrade();
        let clear_btn_weak = clear_btn.downgrade();
        let on_state_changed = on_state_changed.clone();

        remember_row.connect_active_notify(move |row| {
            let active = row.is_active();
            if let Some(auto_row) = auto_open_row_weak.upgrade() {
                auto_row.set_sensitive(active);
            }

            if !active {
                let _ = settings.set_string("last-opened-file", "");
            } else if settings.string("last-opened-file").is_empty() {
                if let Some(ref open_path) = current_open_path {
                    let _ = settings.set_string("last-opened-file", open_path);
                }
            }

            if let Some(pr) = path_row_weak.upgrade() {
                pr.set_subtitle(&format_stored_path(&settings));
            }
            if let Some(cb) = clear_btn_weak.upgrade() {
                cb.set_sensitive(!settings.string("last-opened-file").is_empty());
            }
            on_state_changed();
        });
    }

    // Clear button handler
    {
        let settings = settings.clone();
        let path_row_weak = path_row.downgrade();
        let clear_btn_weak = clear_btn.downgrade();
        let on_state_changed = on_state_changed.clone();

        clear_btn.connect_clicked(move |_| {
            let _ = settings.set_string("last-opened-file", "");
            if let Some(pr) = path_row_weak.upgrade() {
                pr.set_subtitle(&format_stored_path(&settings));
            }
            if let Some(cb) = clear_btn_weak.upgrade() {
                cb.set_sensitive(false);
            }
            on_state_changed();
        });
    }

    dialog.add(&page);
    dialog.present(Some(parent));
}

fn format_stored_path(settings: &gio::Settings) -> String {
    let raw = settings.string("last-opened-file");
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "None".to_string();
    }
    Path::new(trimmed)
        .file_name()
        .map(|f| format!("{} ({})", f.to_string_lossy(), trimmed))
        .unwrap_or_else(|| trimmed.to_string())
}
