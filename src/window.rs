use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use crate::vcard::{parse_vcf, serialize_vcf, VCard};
use crate::widgets::contact_editor::{build_contact_editor, build_contact_preview};
use crate::widgets::preferences_dialog::show_preferences_dialog;

mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct EditBackup {
        pub index: usize,
        pub original_card: VCard,
        pub prev_file_modified: bool,
        pub is_new_contact: bool,
    }

    pub struct TeleponuWindow {
        pub settings: gio::Settings,
        pub toast_overlay: adw::ToastOverlay,
        pub root_stack: gtk::Stack,
        pub recent_btn: gtk::Button,
        pub split_view: adw::OverlaySplitView,
        pub sidebar_header: adw::HeaderBar,
        pub sidebar_toolbar: adw::ToolbarView,
        pub search_entry: gtk::SearchEntry,
        pub contact_list: gtk::ListBox,
        pub detail_stack: gtk::Stack,
        pub editor_container: gtk::Box,
        pub content_header: adw::HeaderBar,
        pub show_sidebar_btn: gtk::ToggleButton,
        pub cancel_btn: gtk::Button,
        pub done_btn: gtk::Button,
        pub edit_btn: gtk::Button,
        pub save_btn: gtk::Button,
        pub add_contact_btn: gtk::Button,

        pub contacts: Rc<RefCell<Vec<VCard>>>,
        pub current_file: Rc<RefCell<Option<PathBuf>>>,
        pub selected_index: Rc<Cell<Option<usize>>>,
        pub is_editing: Rc<Cell<bool>>,
        pub is_modified: Rc<Cell<bool>>,
        pub edit_snapshot: RefCell<Option<EditBackup>>,
        pub editor_validator: RefCell<Option<Rc<dyn Fn() -> bool>>>,
    }

    impl Default for TeleponuWindow {
        fn default() -> Self {
            Self {
                settings: gio::Settings::new("dev.labambangpunto.teleponu"),
                toast_overlay: adw::ToastOverlay::new(),
                root_stack: gtk::Stack::new(),
                recent_btn: gtk::Button::new(),
                split_view: adw::OverlaySplitView::new(),
                sidebar_header: adw::HeaderBar::new(),
                sidebar_toolbar: adw::ToolbarView::new(),
                search_entry: gtk::SearchEntry::new(),
                contact_list: gtk::ListBox::new(),
                detail_stack: gtk::Stack::new(),
                editor_container: gtk::Box::new(gtk::Orientation::Vertical, 0),
                content_header: adw::HeaderBar::new(),
                show_sidebar_btn: gtk::ToggleButton::new(),
                cancel_btn: gtk::Button::new(),
                done_btn: gtk::Button::new(),
                edit_btn: gtk::Button::new(),
                save_btn: gtk::Button::new(),
                add_contact_btn: gtk::Button::new(),
                contacts: Rc::new(RefCell::new(Vec::new())),
                current_file: Rc::new(RefCell::new(None)),
                selected_index: Rc::new(Cell::new(None)),
                is_editing: Rc::new(Cell::new(false)),
                is_modified: Rc::new(Cell::new(false)),
                edit_snapshot: RefCell::new(None),
                editor_validator: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TeleponuWindow {
        const NAME: &'static str = "TeleponuWindow";
        type Type = super::TeleponuWindow;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for TeleponuWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_title(Some("Teleponu"));
            obj.set_default_size(920, 640);

            obj.setup_actions();
            obj.build_ui();
            obj.update_window_title();
            obj.update_recent_button();
            obj.try_auto_open_last_file();
        }
    }

    impl WidgetImpl for TeleponuWindow {}
    impl WindowImpl for TeleponuWindow {
        fn close_request(&self) -> glib::Propagation {
            if self.is_editing.get() {
                return glib::Propagation::Stop;
            }

            let obj = self.obj();
            if self.is_modified.get() {
                obj.confirm_discard_or_save(Rc::new({
                    let obj_weak = obj.downgrade();
                    move || {
                        if let Some(w) = obj_weak.upgrade() {
                            w.imp().is_modified.set(false);
                            w.close();
                        }
                    }
                }));
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    }
    impl ApplicationWindowImpl for TeleponuWindow {}
    impl AdwApplicationWindowImpl for TeleponuWindow {}
}

glib::wrapper! {
    pub struct TeleponuWindow(ObjectSubclass<imp::TeleponuWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TeleponuWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    pub fn show_preferences(&self) {
        let current_open_path = self
            .imp()
            .current_file
            .borrow()
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());

        let win_weak = self.downgrade();
        let on_state_changed = Rc::new(move || {
            if let Some(win) = win_weak.upgrade() {
                win.update_recent_button();
            }
        });

        show_preferences_dialog(
            self,
            &self.imp().settings,
            current_open_path,
            on_state_changed,
        );
    }

    fn setup_actions(&self) {
        let open_action = gio::ActionEntry::builder("open")
            .activate(move |win: &Self, _, _| win.prompt_open_file())
            .build();

        let open_recent_action = gio::ActionEntry::builder("open-recent")
            .activate(move |win: &Self, _, _| win.open_last_remembered_file())
            .build();

        let save_action = gio::ActionEntry::builder("save")
            .activate(move |win: &Self, _, _| win.save_current_file(None))
            .build();

        let save_as_action = gio::ActionEntry::builder("save-as")
            .activate(move |win: &Self, _, _| win.prompt_save_as(None))
            .build();

        let add_contact_action = gio::ActionEntry::builder("add-contact")
            .activate(move |win: &Self, _, _| win.add_new_contact())
            .build();

        self.add_action_entries([
            open_action,
            open_recent_action,
            save_action,
            save_as_action,
            add_contact_action,
        ]);
    }

    fn build_ui(&self) {
        self.load_custom_css();
        let imp = self.imp();

        // === Page 1: Welcome Screen ===
        let welcome_header = adw::HeaderBar::new();
        welcome_header.pack_end(&self.build_primary_menu_button());

        let open_btn = gtk::Button::builder()
            .label("Open File")
            .halign(gtk::Align::Center)
            .css_classes(["pill", "suggested-action"])
            .action_name("win.open")
            .build();

        imp.recent_btn.set_halign(gtk::Align::Center);
        imp.recent_btn.add_css_class("pill");
        imp.recent_btn.set_action_name(Some("win.open-recent"));
        imp.recent_btn.set_visible(false);

        let btn_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .halign(gtk::Align::Center)
            .build();
        btn_box.append(&open_btn);
        btn_box.append(&imp.recent_btn);

        let welcome_status = adw::StatusPage::builder()
            .icon_name("dev.labambangpunto.teleponu")
            .title("Teleponu")
            .description("vCard (VCF) 3.0 Contact File Reader &amp; Editor")
            .child(&btn_box)
            .build();

        let welcome_view = adw::ToolbarView::new();
        welcome_view.add_top_bar(&welcome_header);
        welcome_view.set_content(Some(&welcome_status));

        // === Page 2: Editor Overlay Split View ===
        // 2A. Sidebar (Contact List)
        let sidebar_open_btn = gtk::Button::builder()
            .icon_name("document-open-symbolic")
            .tooltip_text("Open VCF File (Ctrl+O)")
            .action_name("win.open")
            .build();

        imp.sidebar_header.pack_start(&sidebar_open_btn);
        imp.sidebar_header
            .pack_end(&self.build_primary_menu_button());

        imp.search_entry
            .set_placeholder_text(Some("Search name, phone, email..."));
        imp.search_entry.set_margin_start(12);
        imp.search_entry.set_margin_end(12);
        imp.search_entry.set_margin_top(6);
        imp.search_entry.set_margin_bottom(6);

        imp.contact_list
            .set_selection_mode(gtk::SelectionMode::Single);
        imp.contact_list.add_css_class("navigation-sidebar");

        let contacts_for_filter = imp.contacts.clone();
        let search_weak = imp.search_entry.downgrade();
        imp.contact_list.set_filter_func(move |row| {
            let Some(search) = search_weak.upgrade() else {
                return true;
            };
            let query = search.text().trim().to_lowercase();
            if query.is_empty() {
                return true;
            }
            let idx = row.index();
            if idx < 0 {
                return false;
            }
            let list = contacts_for_filter.borrow();
            let Some(card) = list.get(idx as usize) else {
                return false;
            };
            card.fn_name.to_lowercase().contains(&query)
                || card.org.to_lowercase().contains(&query)
                || card.title.to_lowercase().contains(&query)
                || card
                    .phones
                    .iter()
                    .any(|p| p.value.to_lowercase().contains(&query))
                || card
                    .emails
                    .iter()
                    .any(|e| e.value.to_lowercase().contains(&query))
        });

        let list_weak = imp.contact_list.downgrade();
        imp.search_entry.connect_search_changed(move |_| {
            if let Some(list) = list_weak.upgrade() {
                list.invalidate_filter();
            }
        });

        let win_weak = self.downgrade();
        imp.contact_list.connect_row_selected(move |_, row| {
            let Some(win) = win_weak.upgrade() else {
                return;
            };
            if let Some(r) = row {
                let idx = r.index();
                if idx >= 0 {
                    win.select_contact(idx as usize);
                }
            }
        });

        // Add New Contact dashed box at the very bottom of the contact list
        let add_icon = gtk::Image::builder()
            .icon_name("list-add-symbolic")
            .pixel_size(18)
            .build();

        imp.add_contact_btn.set_child(Some(&add_icon));
        imp.add_contact_btn.add_css_class("add-contact-dashed-box");
        imp.add_contact_btn
            .set_tooltip_text(Some("New Contact (Ctrl+N)"));
        imp.add_contact_btn.set_action_name(Some("win.add-contact"));
        imp.add_contact_btn.set_hexpand(true);
        imp.add_contact_btn.set_margin_start(6);
        imp.add_contact_btn.set_margin_end(6);
        imp.add_contact_btn.set_margin_top(4);
        imp.add_contact_btn.set_margin_bottom(8);

        let list_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        list_container.append(&imp.contact_list);
        list_container.append(&imp.add_contact_btn);

        let scrolled_list = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list_container)
            .build();

        let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar_box.append(&imp.search_entry);
        sidebar_box.append(&scrolled_list);

        imp.sidebar_toolbar.add_top_bar(&imp.sidebar_header);
        imp.sidebar_toolbar.set_content(Some(&sidebar_box));

        // 2B. Content (Contact Preview & Editor)
        imp.show_sidebar_btn.set_icon_name("sidebar-show-symbolic");
        imp.show_sidebar_btn.set_tooltip_text(Some("Show Sidebar"));
        imp.show_sidebar_btn.set_visible(false);

        imp.split_view
            .bind_property("show-sidebar", &imp.show_sidebar_btn, "active")
            .bidirectional()
            .sync_create()
            .build();

        imp.cancel_btn.set_label("Cancel");
        imp.cancel_btn.set_tooltip_text(Some("Discard Changes"));
        imp.cancel_btn.set_visible(false);
        {
            let win_weak = self.downgrade();
            imp.cancel_btn.connect_clicked(move |_| {
                if let Some(win) = win_weak.upgrade() {
                    win.cancel_edit_mode();
                }
            });
        }

        imp.done_btn.set_label("Done");
        imp.done_btn.add_css_class("suggested-action");
        imp.done_btn.set_tooltip_text(Some("Done Editing"));
        imp.done_btn.set_visible(false);
        {
            let win_weak = self.downgrade();
            imp.done_btn.connect_clicked(move |_| {
                if let Some(win) = win_weak.upgrade() {
                    win.finish_edit_mode();
                }
            });
        }

        imp.edit_btn.set_icon_name("document-edit-symbolic");
        imp.edit_btn.set_tooltip_text(Some("Edit Contact"));
        imp.edit_btn.set_visible(false);
        {
            let win_weak = self.downgrade();
            imp.edit_btn.connect_clicked(move |_| {
                if let Some(win) = win_weak.upgrade() {
                    win.enter_edit_mode();
                }
            });
        }

        imp.save_btn.set_label("Save changes");
        imp.save_btn.add_css_class("suggested-action");
        imp.save_btn.set_tooltip_text(Some("Save File (Ctrl+S)"));
        imp.save_btn.set_action_name(Some("win.save"));
        imp.save_btn.set_visible(false);

        // Left side of content header: Show Sidebar (when collapsed) or Cancel (when editing)
        imp.content_header.pack_start(&imp.show_sidebar_btn);
        imp.content_header.pack_start(&imp.cancel_btn);

        // Right side of content header: Done (when editing), or Edit + Save changes
        imp.content_header.pack_end(&imp.done_btn);
        imp.content_header.pack_end(&imp.edit_btn);
        imp.content_header.pack_end(&imp.save_btn);

        let empty_detail = adw::StatusPage::builder()
            .icon_name("avatar-default-symbolic")
            .title("No Contact Selected")
            .description("Select a contact from the list on the left or add a new one.")
            .build();

        imp.detail_stack.add_named(&empty_detail, Some("empty"));
        imp.detail_stack
            .add_named(&imp.editor_container, Some("form"));
        imp.detail_stack.set_visible_child_name("empty");

        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&imp.content_header);
        content_toolbar.set_content(Some(&imp.detail_stack));

        imp.split_view.set_sidebar(Some(&imp.sidebar_toolbar));
        imp.split_view.set_content(Some(&content_toolbar));
        imp.split_view.set_min_sidebar_width(250.0);
        imp.split_view.set_max_sidebar_width(320.0);

        {
            let win_weak = self.downgrade();
            imp.split_view.connect_collapsed_notify(move |_| {
                if let Some(win) = win_weak.upgrade() {
                    win.sync_edit_mode_ui();
                }
            });
        }

        // Responsive breakpoint: collapse sidebar into cover overlay on narrow windows
        let condition = adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            650.0,
            adw::LengthUnit::Px,
        );
        let breakpoint = adw::Breakpoint::new(condition);
        {
            let split_view = imp.split_view.clone();
            breakpoint.connect_apply(move |_| {
                split_view.set_collapsed(true);
            });
        }
        {
            let split_view = imp.split_view.clone();
            breakpoint.connect_unapply(move |_| {
                split_view.set_collapsed(false);
            });
        }
        self.add_breakpoint(breakpoint);

        // === Root Stack & ToastOverlay ===
        imp.root_stack.add_named(&welcome_view, Some("welcome"));
        imp.root_stack.add_named(&imp.split_view, Some("editor"));
        imp.root_stack.set_visible_child_name("welcome");

        imp.toast_overlay.set_child(Some(&imp.root_stack));
        self.set_content(Some(&imp.toast_overlay));
    }

    fn load_custom_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(
            "
            button.add-contact-dashed-box {
                background: transparent;
                border: 1.5px dashed alpha(currentColor, 0.35);
                border-radius: 8px;
                min-height: 54px;
                transition: all 150ms ease-in-out;
            }
            button.add-contact-dashed-box:hover {
                background: alpha(currentColor, 0.06);
                border-color: alpha(currentColor, 0.6);
            }
            button.add-contact-dashed-box:active {
                background: alpha(currentColor, 0.12);
            }
            ",
        );

        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn build_primary_menu_button(&self) -> gtk::MenuButton {
        let file_section = gio::Menu::new();
        file_section.append(Some("_New Contact"), Some("win.add-contact"));
        file_section.append(Some("_Open File..."), Some("win.open"));
        file_section.append(Some("_Save"), Some("win.save"));
        file_section.append(Some("Save _As..."), Some("win.save-as"));

        let app_section = gio::Menu::new();
        app_section.append(Some("_Preferences"), Some("app.preferences"));
        app_section.append(Some("_About Teleponu"), Some("app.about"));
        app_section.append(Some("_Quit"), Some("app.quit"));

        let menu = gio::Menu::new();
        menu.append_section(None, &file_section);
        menu.append_section(None, &app_section);

        gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&menu)
            .primary(true)
            .tooltip_text("Main Menu")
            .build()
    }

    fn record_recent_file(&self, path: &Path) {
        let settings = &self.imp().settings;
        if settings.boolean("remember-recent-file") {
            let _ = settings.set_string("last-opened-file", &path.to_string_lossy());
        } else {
            let _ = settings.set_string("last-opened-file", "");
        }
        self.update_recent_button();
    }

    fn update_recent_button(&self) {
        let imp = self.imp();
        if !imp.settings.boolean("remember-recent-file") {
            imp.recent_btn.set_visible(false);
            return;
        }

        let raw = imp.settings.string("last-opened-file");
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            imp.recent_btn.set_visible(false);
            return;
        }

        let file_name = Path::new(trimmed)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| trimmed.to_string());

        imp.recent_btn
            .set_label(&format!("Open Recent ({})", file_name));
        imp.recent_btn.set_tooltip_text(Some(trimmed));
        imp.recent_btn.set_visible(true);
    }

    fn try_auto_open_last_file(&self) {
        let settings = &self.imp().settings;
        if !settings.boolean("remember-recent-file") || !settings.boolean("auto-open-last-file") {
            return;
        }
        let saved = settings.string("last-opened-file");
        let trimmed = saved.trim();
        if trimmed.is_empty() {
            return;
        }
        self.load_vcf_from_path(PathBuf::from(trimmed), true);
    }

    fn open_last_remembered_file(&self) {
        let saved = self.imp().settings.string("last-opened-file");
        let trimmed = saved.trim();
        if trimmed.is_empty() {
            self.show_toast("No recent file path stored.");
            return;
        }
        self.load_vcf_from_path(PathBuf::from(trimmed), false);
    }

    fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        self.imp().toast_overlay.add_toast(toast);
    }

    fn update_window_title(&self) {
        let imp = self.imp();
        let file_label = imp
            .current_file
            .borrow()
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "new-contacts.vcf".to_string());

        let dirty_mark = if imp.is_modified.get() { " •" } else { "" };
        let title = format!("Teleponu — {}{}", file_label, dirty_mark);
        self.set_title(Some(&title));
    }

    fn set_modified(&self, modified: bool) {
        self.imp().is_modified.set(modified);
        self.update_window_title();
        self.sync_edit_mode_ui();
    }

    fn rebuild_contact_list(&self, select_idx: Option<usize>) {
        let imp = self.imp();
        while let Some(child) = imp.contact_list.first_child() {
            imp.contact_list.remove(&child);
        }

        let cards = imp.contacts.borrow().clone();
        for card in &cards {
            let row = self.create_sidebar_row(card);
            imp.contact_list.append(&row);
        }

        if let Some(idx) = select_idx {
            if let Some(row) = imp.contact_list.row_at_index(idx as i32) {
                imp.contact_list.select_row(Some(&row));
            } else {
                *imp.edit_snapshot.borrow_mut() = None;
                imp.selected_index.set(None);
                imp.is_editing.set(false);
                imp.detail_stack.set_visible_child_name("empty");
                self.sync_edit_mode_ui();
            }
        } else {
            *imp.edit_snapshot.borrow_mut() = None;
            imp.selected_index.set(None);
            imp.is_editing.set(false);
            imp.detail_stack.set_visible_child_name("empty");
            self.sync_edit_mode_ui();
        }
    }

    fn create_sidebar_row(&self, card: &VCard) -> adw::ActionRow {
        let disp = card.display_name();
        let sub = card.primary_subtitle();

        let avatar = adw::Avatar::builder()
            .size(38)
            .text(&disp)
            .show_initials(true)
            .build();

        let row = adw::ActionRow::builder()
            .title(&disp)
            .subtitle(&sub)
            .use_markup(false)
            .activatable(true)
            .build();
        row.add_prefix(&avatar);
        row
    }

    fn refresh_sidebar_row(&self, idx: usize) {
        let imp = self.imp();
        let Some(list_row) = imp.contact_list.row_at_index(idx as i32) else {
            return;
        };
        let Some(action_row) = list_row.downcast_ref::<adw::ActionRow>() else {
            return;
        };

        let (disp, sub) = {
            let list = imp.contacts.borrow();
            let Some(c) = list.get(idx) else { return };
            (c.display_name(), c.primary_subtitle())
        };

        action_row.set_title(&disp);
        action_row.set_subtitle(&sub);
        imp.contact_list.invalidate_filter();
    }

    fn select_contact(&self, idx: usize) {
        let imp = self.imp();
        if idx >= imp.contacts.borrow().len() {
            return;
        }

        *imp.edit_snapshot.borrow_mut() = None;
        imp.selected_index.set(Some(idx));
        imp.is_editing.set(false);

        self.render_selected_contact();
        imp.detail_stack.set_visible_child_name("form");

        if imp.split_view.is_collapsed() {
            imp.split_view.set_show_sidebar(false);
        }
    }

    fn enter_edit_mode(&self) {
        let imp = self.imp();
        let Some(idx) = imp.selected_index.get() else {
            return;
        };
        if imp.is_editing.get() {
            return;
        }

        *imp.edit_snapshot.borrow_mut() = Some(imp::EditBackup {
            index: idx,
            original_card: imp.contacts.borrow()[idx].clone(),
            prev_file_modified: imp.is_modified.get(),
            is_new_contact: false,
        });
        imp.is_editing.set(true);
        self.render_selected_contact();
    }

    fn finish_edit_mode(&self) {
        let imp = self.imp();
        let Some(idx) = imp.selected_index.get() else {
            return;
        };
        if !imp.is_editing.get() {
            return;
        }

        let is_valid = if let Some(validator) = imp.editor_validator.borrow().as_ref() {
            validator()
        } else {
            imp.contacts.borrow()[idx].validation_errors().is_empty()
        };

        if !is_valid {
            self.show_toast("Please fix the highlighted formatting errors");
            return;
        }

        *imp.editor_validator.borrow_mut() = None;
        *imp.edit_snapshot.borrow_mut() = None;
        imp.is_editing.set(false);
        self.render_selected_contact();
    }

    fn cancel_edit_mode(&self) {
        let imp = self.imp();
        let Some(idx) = imp.selected_index.get() else {
            return;
        };
        if !imp.is_editing.get() {
            return;
        }

        *imp.editor_validator.borrow_mut() = None;
        let backup = imp.edit_snapshot.borrow_mut().take();
        if let Some(b) = backup {
            if b.index == idx {
                if b.is_new_contact {
                    let next_idx = {
                        let mut list = imp.contacts.borrow_mut();
                        if idx < list.len() {
                            list.remove(idx);
                        }
                        if list.is_empty() {
                            None
                        } else if idx >= list.len() {
                            Some(list.len() - 1)
                        } else {
                            Some(idx)
                        }
                    };
                    self.set_modified(b.prev_file_modified);
                    imp.is_editing.set(false);
                    self.rebuild_contact_list(next_idx);
                    return;
                } else {
                    {
                        let mut list = imp.contacts.borrow_mut();
                        if idx < list.len() {
                            list[idx] = b.original_card;
                        }
                    }
                    self.refresh_sidebar_row(idx);
                    self.set_modified(b.prev_file_modified);
                }
            }
        }

        imp.is_editing.set(false);
        self.render_selected_contact();
    }

    fn sync_edit_mode_ui(&self) {
        let imp = self.imp();
        let editing = imp.is_editing.get();
        let modified = imp.is_modified.get();
        let collapsed = imp.split_view.is_collapsed();
        let has_selection = imp.selected_index.get().is_some();

        // Hide overlay sidebar & disable swipe gestures during edit mode
        if editing && collapsed {
            imp.split_view.set_show_sidebar(false);
        }
        imp.split_view.set_enable_show_gesture(!editing);

        // Dim & disable all sidebar components during edit mode
        imp.sidebar_toolbar.set_sensitive(!editing);

        // Hide window close/title buttons during edit mode
        imp.sidebar_header.set_show_start_title_buttons(!editing);
        imp.sidebar_header.set_show_end_title_buttons(!editing);
        imp.content_header.set_show_start_title_buttons(!editing);
        imp.content_header.set_show_end_title_buttons(!editing);

        // Top bar buttons
        imp.show_sidebar_btn.set_visible(!editing && collapsed);
        imp.cancel_btn.set_visible(editing);
        imp.done_btn.set_visible(editing);
        imp.edit_btn.set_visible(!editing && has_selection);
        imp.save_btn.set_visible(!editing && modified);

        for action_name in ["open", "open-recent", "save-as", "add-contact"] {
            if let Some(action) = self.lookup_action(action_name) {
                if let Some(simple_action) = action.downcast_ref::<gio::SimpleAction>() {
                    simple_action.set_enabled(!editing);
                }
            }
        }

        if let Some(action) = self.lookup_action("save") {
            if let Some(simple_action) = action.downcast_ref::<gio::SimpleAction>() {
                simple_action.set_enabled(!editing && modified);
            }
        }
    }

    fn render_selected_contact(&self) {
        let imp = self.imp();
        let Some(idx) = imp.selected_index.get() else {
            *imp.editor_validator.borrow_mut() = None;
            imp.is_editing.set(false);
            imp.detail_stack.set_visible_child_name("empty");
            self.sync_edit_mode_ui();
            return;
        };

        self.sync_edit_mode_ui();

        while let Some(child) = imp.editor_container.first_child() {
            imp.editor_container.remove(&child);
        }

        if imp.is_editing.get() {
            let on_modified: Rc<dyn Fn()> = {
                let win_weak = self.downgrade();
                Rc::new(move || {
                    if let Some(win) = win_weak.upgrade() {
                        win.refresh_sidebar_row(idx);
                        win.set_modified(true);
                    }
                })
            };

            let on_delete_contact: Rc<dyn Fn(usize)> = {
                let win_weak = self.downgrade();
                Rc::new(move |target_idx| {
                    if let Some(win) = win_weak.upgrade() {
                        win.delete_contact(target_idx);
                    }
                })
            };

            let editor_view =
                build_contact_editor(imp.contacts.clone(), idx, on_modified, on_delete_contact);
            *imp.editor_validator.borrow_mut() = Some(editor_view.validate_and_highlight);
            imp.editor_container.append(&editor_view.widget);
        } else {
            *imp.editor_validator.borrow_mut() = None;
            let card = imp.contacts.borrow()[idx].clone();
            let on_copied: Rc<dyn Fn(&str)> = {
                let win_weak = self.downgrade();
                Rc::new(move |_| {
                    if let Some(win) = win_weak.upgrade() {
                        win.show_toast("Copied to clipboard");
                    }
                })
            };

            let preview = build_contact_preview(&card, on_copied);
            imp.editor_container.append(&preview);
        }
    }

    fn add_new_contact(&self) {
        let imp = self.imp();
        if imp.is_editing.get() {
            return;
        }

        let prev_modified = imp.is_modified.get();
        let blank_card = VCard::new_blank();
        let new_idx = {
            let mut list = imp.contacts.borrow_mut();
            list.push(blank_card.clone());
            list.len() - 1
        };

        imp.root_stack.set_visible_child_name("editor");
        self.set_modified(true);
        self.rebuild_contact_list(Some(new_idx));

        *imp.edit_snapshot.borrow_mut() = Some(imp::EditBackup {
            index: new_idx,
            original_card: blank_card,
            prev_file_modified: prev_modified,
            is_new_contact: true,
        });

        imp.is_editing.set(true);
        self.render_selected_contact();
        self.show_toast("New contact added");
    }

    fn delete_contact(&self, idx: usize) {
        let imp = self.imp();
        *imp.edit_snapshot.borrow_mut() = None;
        imp.is_editing.set(false);

        let next_idx = {
            let mut list = imp.contacts.borrow_mut();
            if idx >= list.len() {
                return;
            }
            list.remove(idx);
            if list.is_empty() {
                None
            } else if idx >= list.len() {
                Some(list.len() - 1)
            } else {
                Some(idx)
            }
        };

        self.set_modified(true);
        self.rebuild_contact_list(next_idx);
        self.show_toast("Contact deleted");
    }

    fn prompt_open_file(&self) {
        let win_weak = self.downgrade();
        let action = Rc::new(move || {
            let Some(win) = win_weak.upgrade() else {
                return;
            };
            win.open_file_dialog();
        });

        if self.imp().is_modified.get() {
            self.confirm_discard_or_save(action);
        } else {
            action();
        }
    }

    fn open_file_dialog(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("vCard 3.0 Files (*.vcf)"));
        filter.add_pattern("*.vcf");
        filter.add_mime_type("text/vcard");

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Open VCF 3.0 File")
            .filters(&filters)
            .default_filter(&filter)
            .modal(true)
            .build();

        let win_weak = self.downgrade();
        dialog.open(Some(self), gio::Cancellable::NONE, move |result| {
            let Some(win) = win_weak.upgrade() else {
                return;
            };
            let Ok(file) = result else { return };
            let Some(path) = file.path() else {
                win.show_toast("Failed to resolve file path.");
                return;
            };

            win.load_vcf_from_path(path, false);
        });
    }

    fn load_vcf_from_path(&self, path: PathBuf, is_startup: bool) {
        if !path.exists() {
            let _ = self.imp().settings.set_string("last-opened-file", "");
            self.update_recent_button();
            if is_startup {
                self.show_toast("Previous recent file was not found");
            } else {
                self.show_error_dialog("File Not Found", "The selected VCF file no longer exists.");
            }
            return;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match parse_vcf(&content) {
                Ok(cards) => {
                    let count = cards.len();
                    let imp = self.imp();
                    *imp.contacts.borrow_mut() = cards;
                    *imp.current_file.borrow_mut() = Some(path.clone());
                    imp.root_stack.set_visible_child_name("editor");
                    self.set_modified(false);
                    self.record_recent_file(&path);
                    self.rebuild_contact_list(if count > 0 { Some(0) } else { None });
                    let msg = if count == 1 {
                        "1 contact loaded".to_string()
                    } else {
                        format!("{} contacts loaded", count)
                    };
                    self.show_toast(&msg);
                }
                Err(err) => {
                    if is_startup {
                        self.show_toast("Failed to auto-open last file");
                    } else {
                        self.show_error_dialog("Failed to Read VCF 3.0", &err.to_string());
                    }
                }
            },
            Err(err) => {
                if is_startup {
                    self.show_toast("Failed to auto-open last file");
                } else {
                    self.show_error_dialog("Failed to Open File", &err.to_string());
                }
            }
        }
    }

    fn collect_all_validation_warnings(&self) -> Vec<String> {
        let list = self.imp().contacts.borrow();
        let mut all = Vec::new();
        for card in list.iter() {
            let name = card.display_name();
            for err in card.validation_errors() {
                all.push(format!("• {} — {}", name, err));
            }
        }
        all
    }

    fn show_validation_block_dialog(&self, heading: &str, intro: &str, warnings: Vec<String>) {
        let body = format!("{}\n\n{}", intro, warnings.join("\n"));

        let dialog = adw::AlertDialog::builder()
            .heading(heading)
            .body(&body)
            .build();

        dialog.add_response("ok", "Go Back & Fix");
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("ok");

        dialog.present(Some(self));
    }

    fn save_current_file(&self, after_save: Option<Rc<dyn Fn()>>) {
        let warnings = self.collect_all_validation_warnings();
        if !warnings.is_empty() {
            self.show_validation_block_dialog(
                "Cannot Save File",
                "The file cannot be saved until the following formatting errors are fixed:",
                warnings,
            );
            return;
        }
        self.execute_save_current_file(after_save);
    }

    fn execute_save_current_file(&self, after_save: Option<Rc<dyn Fn()>>) {
        let existing_path = self.imp().current_file.borrow().clone();
        match existing_path {
            Some(path) => {
                if self.write_vcf_to_path(&path) {
                    if let Some(cb) = after_save {
                        cb();
                    }
                }
            }
            None => self.execute_prompt_save_as(after_save),
        }
    }

    fn prompt_save_as(&self, after_save: Option<Rc<dyn Fn()>>) {
        let warnings = self.collect_all_validation_warnings();
        if !warnings.is_empty() {
            self.show_validation_block_dialog(
                "Cannot Save File",
                "The file cannot be saved until the following formatting errors are fixed:",
                warnings,
            );
            return;
        }
        self.execute_prompt_save_as(after_save);
    }

    fn execute_prompt_save_as(&self, after_save: Option<Rc<dyn Fn()>>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("vCard 3.0 Files (*.vcf)"));
        filter.add_pattern("*.vcf");

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let initial_name = self
            .imp()
            .current_file
            .borrow()
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "contacts.vcf".to_string());

        let dialog = gtk::FileDialog::builder()
            .title("Save VCF 3.0 File")
            .initial_name(&initial_name)
            .filters(&filters)
            .default_filter(&filter)
            .modal(true)
            .build();

        let win_weak = self.downgrade();
        dialog.save(Some(self), gio::Cancellable::NONE, move |result| {
            let Some(win) = win_weak.upgrade() else {
                return;
            };
            let Ok(file) = result else { return };
            let Some(mut path) = file.path() else {
                win.show_toast("Failed to determine save destination.");
                return;
            };

            if path.extension().is_none() {
                path.set_extension("vcf");
            }

            if win.write_vcf_to_path(&path) {
                if let Some(cb) = after_save {
                    cb();
                }
            }
        });
    }

    fn write_vcf_to_path(&self, path: &PathBuf) -> bool {
        let serialized = {
            let list = self.imp().contacts.borrow();
            serialize_vcf(&list)
        };

        match std::fs::write(path, serialized) {
            Ok(()) => {
                *self.imp().current_file.borrow_mut() = Some(path.clone());
                self.set_modified(false);
                self.record_recent_file(path);
                self.show_toast("VCF 3.0 file saved");
                true
            }
            Err(err) => {
                self.show_error_dialog("Failed to Save File", &err.to_string());
                false
            }
        }
    }

    fn show_error_dialog(&self, heading: &str, body: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading(heading)
            .body(body)
            .build();
        dialog.add_response("ok", "Close");
        dialog.present(Some(self));
    }

    fn confirm_discard_or_save(&self, proceed: Rc<dyn Fn()>) {
        let dialog = adw::AlertDialog::builder()
            .heading("Save Changes?")
            .body("There are unsaved changes to the contact list.")
            .build();

        dialog.add_responses(&[
            ("cancel", "Cancel"),
            ("discard", "Discard"),
            ("save", "Save"),
        ]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");

        let win_weak = self.downgrade();
        dialog.choose(self, gio::Cancellable::NONE, move |response| {
            let Some(win) = win_weak.upgrade() else {
                return;
            };
            match response.as_str() {
                "discard" => {
                    win.set_modified(false);
                    proceed();
                }
                "save" => {
                    win.save_current_file(Some(proceed));
                }
                _ => {}
            }
        });
    }
}

