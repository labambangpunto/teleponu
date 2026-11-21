use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio};

use crate::vcard::{
    validate_email, validate_full_name, validate_phone, validate_postal_code, validate_url,
    Address, TypedValue, VCard, MAX_FULL_NAME_LEN,
};

pub fn build_contact_preview(card: &VCard, on_copied: Rc<dyn Fn(&str)>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();

    // --- Contact Profile Header ---
    let header_group = adw::PreferencesGroup::new();
    let header_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .halign(gtk::Align::Center)
        .margin_top(12)
        .margin_bottom(8)
        .build();

    let avatar = adw::Avatar::builder()
        .size(84)
        .text(&card.display_name())
        .show_initials(true)
        .build();

    let name_heading = gtk::Label::builder()
        .label(&card.display_name())
        .css_classes(["title-1"])
        .wrap(true)
        .selectable(true)
        .justify(gtk::Justification::Center)
        .build();

    header_box.append(&avatar);
    header_box.append(&name_heading);

    if let Some(role_text) = format_role_subtitle(card) {
        let sub_heading = gtk::Label::builder()
            .label(&role_text)
            .css_classes(["dim-label"])
            .wrap(true)
            .selectable(true)
            .justify(gtk::Justification::Center)
            .build();
        header_box.append(&sub_heading);
    }

    header_group.add(&header_box);
    page.add(&header_group);

    let mut has_any_detail = false;

    // --- Phone Numbers ---
    let valid_phones: Vec<&TypedValue> = card
        .phones
        .iter()
        .filter(|p| !p.value.trim().is_empty())
        .collect();
    if !valid_phones.is_empty() {
        has_any_detail = true;
        let group = adw::PreferencesGroup::new();
        for phone in valid_phones {
            let icon_name = phone_type_icon(&phone.types);
            let tooltip = format_phone_type_label(&phone.types);
            let row = build_preview_copy_row(
                Some((icon_name, &tooltip)),
                phone.value.trim(),
                on_copied.clone(),
            );
            group.add(&row);
        }
        page.add(&group);
    }

    // --- Email Addresses ---
    let valid_emails: Vec<&TypedValue> = card
        .emails
        .iter()
        .filter(|e| !e.value.trim().is_empty())
        .collect();
    if !valid_emails.is_empty() {
        has_any_detail = true;
        let group = adw::PreferencesGroup::new();
        for email in valid_emails {
            let icon_name = email_type_icon(&email.types);
            let tooltip = format_email_type_label(&email.types);
            let email_addr = email.value.trim();

            let row =
                build_preview_copy_row(Some((icon_name, &tooltip)), email_addr, on_copied.clone());

            let send_btn = build_preview_uri_button(
                "mail-send-symbolic",
                &format!("Send an email to {}", email_addr),
                format!("mailto:{}", email_addr),
            );
            row.add_suffix(&send_btn);

            group.add(&row);
        }
        page.add(&group);
    }

    // --- Websites ---
    let valid_urls: Vec<&String> = card.urls.iter().filter(|u| !u.trim().is_empty()).collect();
    if !valid_urls.is_empty() {
        has_any_detail = true;
        let group = adw::PreferencesGroup::new();
        for url in valid_urls {
            let url_str = url.trim();
            let row = build_preview_copy_row(None, url_str, on_copied.clone());

            let target_uri = if url_str.to_ascii_lowercase().starts_with("http://")
                || url_str.to_ascii_lowercase().starts_with("https://")
            {
                url_str.to_string()
            } else {
                format!("https://{}", url_str)
            };

            let visit_btn =
                build_preview_uri_button("external-link-symbolic", "Visit website", target_uri);
            row.add_suffix(&visit_btn);

            group.add(&row);
        }
        page.add(&group);
    }

    // --- Addresses ---
    let valid_addrs: Vec<&Address> = card
        .addresses
        .iter()
        .filter(|a| {
            [
                &a.po_box,
                &a.extended,
                &a.street,
                &a.city,
                &a.region,
                &a.postal_code,
                &a.country,
            ]
            .iter()
            .any(|f| !f.trim().is_empty())
        })
        .collect();
    if !valid_addrs.is_empty() {
        has_any_detail = true;
        let group = adw::PreferencesGroup::new();
        for addr in valid_addrs {
            let icon_name = address_type_icon(&addr.types);
            let tooltip = format_address_type_label(&addr.types);
            let summary = addr.summary();
            let row =
                build_preview_copy_row(Some((icon_name, &tooltip)), &summary, on_copied.clone());
            group.add(&row);
        }
        page.add(&group);
    }

    // --- Work & Company Info ---
    let has_work_info = !card.org.trim().is_empty() || !card.title.trim().is_empty();
    if has_work_info {
        has_any_detail = true;
        let group = adw::PreferencesGroup::new();

        if !card.org.trim().is_empty() {
            group.add(&build_preview_info_row(
                "network-workgroup-symbolic",
                "Company",
                card.org.trim(),
            ));
        }
        if !card.title.trim().is_empty() {
            group.add(&build_preview_info_row(
                "avatar-default-symbolic",
                "Role",
                card.title.trim(),
            ));
        }

        page.add(&group);
    }

    // --- Notes ---
    if !card.note.trim().is_empty() {
        has_any_detail = true;
        let group = adw::PreferencesGroup::new();

        let note_label = gtk::Label::builder()
            .label(card.note.trim())
            .wrap(true)
            .xalign(0.0)
            .selectable(true)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(14)
            .margin_end(14)
            .build();

        let note_card = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        note_card.append(&note_label);

        group.add(&note_card);
        page.add(&group);
    }

    if !has_any_detail {
        let empty_group = adw::PreferencesGroup::new();
        let empty_row = adw::ActionRow::builder()
            .title("No contact details")
            .subtitle("Click the edit button in the top-right corner to add information.")
            .use_markup(false)
            .build();
        empty_group.add(&empty_row);
        page.add(&empty_group);
    }

    page
}

fn build_preview_info_row(icon_name: &str, tooltip: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(value)
        .use_markup(false)
        .build();

    let icon = gtk::Image::builder()
        .icon_name(icon_name)
        .tooltip_text(tooltip)
        .css_classes(["dim-label"])
        .valign(gtk::Align::Center)
        .build();
    row.add_prefix(&icon);

    row
}

fn build_preview_copy_row(
    prefix_icon: Option<(&str, &str)>,
    value: &str,
    on_copied: Rc<dyn Fn(&str)>,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(value)
        .use_markup(false)
        .build();

    if let Some((icon_name, tooltip)) = prefix_icon {
        let icon = gtk::Image::builder()
            .icon_name(icon_name)
            .tooltip_text(tooltip)
            .css_classes(["dim-label"])
            .valign(gtk::Align::Center)
            .build();
        row.add_prefix(&icon);
    }

    let copy_btn = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .tooltip_text("Copy to Clipboard")
        .build();

    let val_owned = value.to_string();
    copy_btn.connect_clicked(move |_| {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(&val_owned);
            on_copied(&val_owned);
        }
    });

    row.add_suffix(&copy_btn);
    row
}

fn build_preview_uri_button(icon_name: &str, tooltip: &str, uri: String) -> gtk::Button {
    let btn = gtk::Button::builder()
        .icon_name(icon_name)
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .tooltip_text(tooltip)
        .build();

    btn.connect_clicked(move |b| {
        let launcher = gtk::UriLauncher::new(&uri);
        let parent_win = b.root().and_downcast::<gtk::Window>();
        launcher.launch(parent_win.as_ref(), gio::Cancellable::NONE, |_| {});
    });

    btn
}

fn phone_type_icon(types: &[String]) -> &'static str {
    let has = |needle: &str| types.iter().any(|t| t.eq_ignore_ascii_case(needle));

    if has("FAX") {
        "printer-symbolic"
    } else if has("CELL") || has("MOBILE") {
        "phone-symbolic"
    } else if has("HOME") {
        "go-home-symbolic"
    } else {
        "call-start-symbolic"
    }
}

fn email_type_icon(types: &[String]) -> &'static str {
    let has = |needle: &str| types.iter().any(|t| t.eq_ignore_ascii_case(needle));

    if has("WORK") {
        "network-workgroup-symbolic"
    } else if has("HOME") {
        "go-home-symbolic"
    } else if has("PREF") {
        "starred-symbolic"
    } else {
        "mail-unread-symbolic"
    }
}

fn address_type_icon(types: &[String]) -> &'static str {
    let has = |needle: &str| types.iter().any(|t| t.eq_ignore_ascii_case(needle));

    if has("WORK") {
        "network-workgroup-symbolic"
    } else if has("HOME") {
        "go-home-symbolic"
    } else {
        "mark-location-symbolic"
    }
}

pub struct ContactEditorView {
    pub widget: adw::PreferencesPage,
    pub validate_and_highlight: Rc<dyn Fn() -> bool>,
}

pub fn build_contact_editor(
    contacts: Rc<RefCell<Vec<VCard>>>,
    index: usize,
    on_modified: Rc<dyn Fn()>,
    on_delete_contact: Rc<dyn Fn(usize)>,
) -> ContactEditorView {
    let page = adw::PreferencesPage::new();
    let show_validation = Rc::new(Cell::new(false));

    let initial_card = {
        let list = contacts.borrow();
        list[index].clone()
    };

    // --- Validation Warning Card (hidden until "Done" is clicked with invalid fields) ---
    let warning_group = adw::PreferencesGroup::builder().visible(false).build();

    let warning_icon = gtk::Image::builder()
        .icon_name("dialog-warning-symbolic")
        .css_classes(["error"])
        .valign(gtk::Align::Center)
        .build();

    let warning_row = adw::ActionRow::builder()
        .title("Format warning")
        .css_classes(["error"])
        .use_markup(false)
        .build();
    warning_row.add_prefix(&warning_icon);
    warning_group.add(&warning_row);
    page.add(&warning_group);

    let refresh_validation_banner: Rc<dyn Fn()> = {
        let contacts = contacts.clone();
        let warning_group = warning_group.clone();
        let warning_row = warning_row.clone();
        let show_validation = show_validation.clone();
        Rc::new(move || {
            if !show_validation.get() {
                warning_group.set_visible(false);
                return;
            }

            let errors = contacts.borrow()[index].validation_errors();
            if errors.is_empty() {
                warning_group.set_visible(false);
            } else {
                warning_row.set_title(&errors[0]);
                if errors.len() > 1 {
                    warning_row.set_subtitle(&format!(
                        "And {} more formatting error(s) highlighted below.",
                        errors.len() - 1
                    ));
                } else {
                    warning_row.set_subtitle("Fix the highlighted field below, then press Done.");
                }
                warning_group.set_visible(true);
            }
        })
    };

    // --- Contact Profile Header (Avatar + Name + Role/Company) ---
    let header_group = adw::PreferencesGroup::new();
    let header_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .halign(gtk::Align::Center)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    let avatar = adw::Avatar::builder()
        .size(72)
        .text(&initial_card.display_name())
        .show_initials(true)
        .build();

    let name_heading = gtk::Label::builder()
        .label(&initial_card.display_name())
        .css_classes(["title-1"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    let initial_sub = format_role_subtitle(&initial_card);
    let sub_heading = gtk::Label::builder()
        .label(initial_sub.as_deref().unwrap_or(""))
        .visible(initial_sub.is_some())
        .css_classes(["dim-label"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    header_box.append(&avatar);
    header_box.append(&name_heading);
    header_box.append(&sub_heading);
    header_group.add(&header_box);
    page.add(&header_group);

    let refresh_header = {
        let contacts = contacts.clone();
        let avatar = avatar.clone();
        let name_heading = name_heading.clone();
        let sub_heading = sub_heading.clone();
        move || {
            let (disp, sub) = {
                let list = contacts.borrow();
                let c = &list[index];
                (c.display_name(), format_role_subtitle(c))
            };
            avatar.set_text(Some(&disp));
            name_heading.set_label(&disp);
            match sub {
                Some(text) => {
                    sub_heading.set_label(&text);
                    sub_heading.set_visible(true);
                }
                None => {
                    sub_heading.set_visible(false);
                }
            }
        }
    };

    // --- Group 1: Identity ---
    let identity_group = adw::PreferencesGroup::builder().title("Identity").build();

    let given_row = adw::EntryRow::builder()
        .title("First Name")
        .text(&initial_card.n.given)
        .build();
    let given_warn_icon = create_row_warning_icon();
    given_row.add_suffix(&given_warn_icon);

    let family_row = adw::EntryRow::builder()
        .title("Last Name")
        .text(&initial_card.n.family)
        .build();
    let family_warn_icon = create_row_warning_icon();
    family_row.add_suffix(&family_warn_icon);

    let middle_row = adw::EntryRow::builder()
        .title("Middle Name")
        .text(&initial_card.n.middle)
        .build();
    let middle_warn_icon = create_row_warning_icon();
    middle_row.add_suffix(&middle_warn_icon);

    let prefix_row = adw::EntryRow::builder()
        .title("Prefix")
        .text(&initial_card.n.prefix)
        .build();
    let prefix_warn_icon = create_row_warning_icon();
    prefix_row.add_suffix(&prefix_warn_icon);

    let suffix_row = adw::EntryRow::builder()
        .title("Suffix")
        .text(&initial_card.n.suffix)
        .build();
    let suffix_warn_icon = create_row_warning_icon();
    suffix_row.add_suffix(&suffix_warn_icon);

    let org_row = adw::EntryRow::builder()
        .title("Company")
        .text(&initial_card.org)
        .build();
    let title_row = adw::EntryRow::builder()
        .title("Role")
        .text(&initial_card.title)
        .build();

    let refresh_identity_ui: Rc<dyn Fn()> = {
        let contacts = contacts.clone();
        let identity_group = identity_group.clone();
        let show_validation = show_validation.clone();
        let given_row = given_row.clone();
        let given_warn_icon = given_warn_icon.clone();
        let family_row = family_row.clone();
        let family_warn_icon = family_warn_icon.clone();
        let middle_row = middle_row.clone();
        let middle_warn_icon = middle_warn_icon.clone();
        let prefix_row = prefix_row.clone();
        let prefix_warn_icon = prefix_warn_icon.clone();
        let suffix_row = suffix_row.clone();
        let suffix_warn_icon = suffix_warn_icon.clone();

        Rc::new(move || {
            let (effective_name, err) = {
                let list = contacts.borrow();
                let card = &list[index];
                let from_n = card.n.to_full_name();
                let name = if from_n.is_empty() {
                    card.fn_name.trim().to_string()
                } else {
                    from_n
                };
                let validation = validate_full_name(&name);
                (name, validation)
            };

            let char_count = effective_name.chars().count();
            if effective_name.is_empty() {
                identity_group.set_description(Some(&format!(
                    "Maximum {} characters including spaces (0/{} characters)",
                    MAX_FULL_NAME_LEN, MAX_FULL_NAME_LEN
                )));
            } else {
                identity_group.set_description(Some(&format!(
                    "Combined name: \"{}\" ({}/{} characters)",
                    effective_name, char_count, MAX_FULL_NAME_LEN
                )));
            }

            let all_rows = [
                (&given_row, &given_warn_icon),
                (&family_row, &family_warn_icon),
                (&middle_row, &middle_warn_icon),
                (&prefix_row, &prefix_warn_icon),
                (&suffix_row, &suffix_warn_icon),
            ];

            if !show_validation.get() {
                for (row, icon) in all_rows {
                    apply_row_validation(row, icon, None);
                }
                return;
            }

            match err {
                Some(msg) if effective_name.is_empty() => {
                    apply_row_validation(&given_row, &given_warn_icon, Some(msg));
                    apply_row_validation(&family_row, &family_warn_icon, Some(msg));
                    apply_row_validation(&middle_row, &middle_warn_icon, None);
                    apply_row_validation(&prefix_row, &prefix_warn_icon, None);
                    apply_row_validation(&suffix_row, &suffix_warn_icon, None);
                }
                Some(msg) => {
                    for (row, icon) in all_rows {
                        let row_err = if row.text().trim().is_empty() {
                            None
                        } else {
                            Some(msg)
                        };
                        apply_row_validation(row, icon, row_err);
                    }
                }
                None => {
                    for (row, icon) in all_rows {
                        apply_row_validation(row, icon, None);
                    }
                }
            }
        })
    };

    let sync_n_field = |field_setter: fn(&mut VCard, String)| {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let refresh_header = refresh_header.clone();
        let refresh_identity = refresh_identity_ui.clone();
        let refresh_banner = refresh_validation_banner.clone();

        move |r: &adw::EntryRow| {
            let mut val = r.text().to_string();
            let was_truncated = {
                let mut list = contacts.borrow_mut();
                let card = &mut list[index];
                field_setter(card, val.clone());

                let mut truncated = false;
                while !val.is_empty()
                    && (card.n.to_full_name().chars().count() > MAX_FULL_NAME_LEN
                        || val.chars().count() > MAX_FULL_NAME_LEN)
                {
                    val.pop();
                    field_setter(card, val.clone());
                    truncated = true;
                }

                card.fn_name = card.n.to_full_name();
                truncated
            };

            if was_truncated {
                r.set_text(&val);
                r.set_position(-1);
                return;
            }

            refresh_header();
            refresh_identity();
            refresh_banner();
            on_modified();
        }
    };

    given_row.connect_changed(sync_n_field(|c, v| c.n.given = v));
    family_row.connect_changed(sync_n_field(|c, v| c.n.family = v));
    middle_row.connect_changed(sync_n_field(|c, v| c.n.middle = v));
    prefix_row.connect_changed(sync_n_field(|c, v| c.n.prefix = v));
    suffix_row.connect_changed(sync_n_field(|c, v| c.n.suffix = v));

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let refresh_header = refresh_header.clone();
        org_row.connect_changed(move |r| {
            contacts.borrow_mut()[index].org = r.text().to_string();
            refresh_header();
            on_modified();
        });
    }

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let refresh_header = refresh_header.clone();
        title_row.connect_changed(move |r| {
            contacts.borrow_mut()[index].title = r.text().to_string();
            refresh_header();
            on_modified();
        });
    }

    identity_group.add(&given_row);
    identity_group.add(&family_row);
    identity_group.add(&middle_row);
    identity_group.add(&prefix_row);
    identity_group.add(&suffix_row);
    identity_group.add(&org_row);
    identity_group.add(&title_row);
    page.add(&identity_group);

    refresh_identity_ui();

    // --- Group 2: Phone Numbers ---
    let add_phone_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .label("Add")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let phones_group = adw::PreferencesGroup::builder()
        .title("Phone")
        .description("Include country code (e.g., +628123456789)")
        .header_suffix(&add_phone_btn)
        .build();

    let phone_rows_tracker: Rc<RefCell<Vec<gtk::Widget>>> = Rc::new(RefCell::new(Vec::new()));
    let rebuild_phones: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    {
        let contacts = contacts.clone();
        let phones_group = phones_group.clone();
        let phone_rows_tracker = phone_rows_tracker.clone();
        let on_modified = on_modified.clone();
        let rebuild_phones_ref = rebuild_phones.clone();
        let refresh_banner = refresh_validation_banner.clone();
        let show_validation = show_validation.clone();

        let closure: Rc<dyn Fn()> = Rc::new(move || {
            for w in phone_rows_tracker.borrow_mut().drain(..) {
                phones_group.remove(&w);
            }

            let count = contacts.borrow()[index].phones.len();
            if count == 0 {
                let empty = adw::ActionRow::builder()
                    .title("No phone numbers")
                    .use_markup(false)
                    .build();
                phones_group.add(&empty);
                phone_rows_tracker.borrow_mut().push(empty.upcast());
                return;
            }

            for p_idx in 0..count {
                let item = contacts.borrow()[index].phones[p_idx].clone();
                let row = build_typed_value_row(
                    "Phone Number (+62...)",
                    &item,
                    &[
                        "CELL,VOICE",
                        "WORK,VOICE",
                        "HOME,VOICE",
                        "CELL",
                        "WORK",
                        "HOME",
                    ],
                    validate_phone,
                    show_validation.clone(),
                    {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        move |new_types| {
                            contacts.borrow_mut()[index].phones[p_idx]
                                .set_types_from_str(&new_types);
                            on_modified();
                        }
                    },
                    {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        let refresh_banner = refresh_banner.clone();
                        move |new_val| {
                            contacts.borrow_mut()[index].phones[p_idx].value = new_val;
                            refresh_banner();
                            on_modified();
                        }
                    },
                    {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        let rebuild_phones_ref = rebuild_phones_ref.clone();
                        let refresh_banner = refresh_banner.clone();
                        move || {
                            contacts.borrow_mut()[index].phones.remove(p_idx);
                            refresh_banner();
                            on_modified();
                            if let Some(cb) = rebuild_phones_ref.borrow().as_ref() {
                                cb();
                            }
                        }
                    },
                );
                phones_group.add(&row);
                phone_rows_tracker.borrow_mut().push(row.upcast());
            }
        });

        closure();
        *rebuild_phones.borrow_mut() = Some(closure);
    }

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let rebuild_phones = rebuild_phones.clone();
        let refresh_banner = refresh_validation_banner.clone();
        add_phone_btn.connect_clicked(move |_| {
            contacts.borrow_mut()[index].phones.push(TypedValue {
                types: vec!["CELL".to_string(), "VOICE".to_string()],
                value: String::new(),
            });
            refresh_banner();
            on_modified();
            if let Some(cb) = rebuild_phones.borrow().as_ref() {
                cb();
            }
        });
    }
    page.add(&phones_group);

    // --- Group 3: Email Addresses ---
    let add_email_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .label("Add")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let emails_group = adw::PreferencesGroup::builder()
        .title("Email")
        .header_suffix(&add_email_btn)
        .build();

    let email_rows_tracker: Rc<RefCell<Vec<gtk::Widget>>> = Rc::new(RefCell::new(Vec::new()));
    let rebuild_emails: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    {
        let contacts = contacts.clone();
        let emails_group = emails_group.clone();
        let email_rows_tracker = email_rows_tracker.clone();
        let on_modified = on_modified.clone();
        let rebuild_emails_ref = rebuild_emails.clone();
        let refresh_banner = refresh_validation_banner.clone();
        let show_validation = show_validation.clone();

        let closure: Rc<dyn Fn()> = Rc::new(move || {
            for w in email_rows_tracker.borrow_mut().drain(..) {
                emails_group.remove(&w);
            }

            let count = contacts.borrow()[index].emails.len();
            if count == 0 {
                let empty = adw::ActionRow::builder()
                    .title("No email addresses")
                    .use_markup(false)
                    .build();
                emails_group.add(&empty);
                email_rows_tracker.borrow_mut().push(empty.upcast());
                return;
            }

            for e_idx in 0..count {
                let item = contacts.borrow()[index].emails[e_idx].clone();
                let row = build_typed_value_row(
                    "Email Address (name@example.com)",
                    &item,
                    &[
                        "PREF,INTERNET",
                        "WORK,INTERNET",
                        "HOME,INTERNET",
                        "INTERNET",
                    ],
                    validate_email,
                    show_validation.clone(),
                    {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        move |new_types| {
                            contacts.borrow_mut()[index].emails[e_idx]
                                .set_types_from_str(&new_types);
                            on_modified();
                        }
                    },
                    {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        let refresh_banner = refresh_banner.clone();
                        move |new_val| {
                            contacts.borrow_mut()[index].emails[e_idx].value = new_val;
                            refresh_banner();
                            on_modified();
                        }
                    },
                    {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        let rebuild_emails_ref = rebuild_emails_ref.clone();
                        let refresh_banner = refresh_banner.clone();
                        move || {
                            contacts.borrow_mut()[index].emails.remove(e_idx);
                            refresh_banner();
                            on_modified();
                            if let Some(cb) = rebuild_emails_ref.borrow().as_ref() {
                                cb();
                            }
                        }
                    },
                );
                emails_group.add(&row);
                email_rows_tracker.borrow_mut().push(row.upcast());
            }
        });

        closure();
        *rebuild_emails.borrow_mut() = Some(closure);
    }

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let rebuild_emails = rebuild_emails.clone();
        let refresh_banner = refresh_validation_banner.clone();
        add_email_btn.connect_clicked(move |_| {
            contacts.borrow_mut()[index].emails.push(TypedValue {
                types: vec!["PREF".to_string(), "INTERNET".to_string()],
                value: String::new(),
            });
            refresh_banner();
            on_modified();
            if let Some(cb) = rebuild_emails.borrow().as_ref() {
                cb();
            }
        });
    }
    page.add(&emails_group);

    // --- Group 4: Websites ---
    let add_url_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .label("Add")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let urls_group = adw::PreferencesGroup::builder()
        .title("Website")
        .header_suffix(&add_url_btn)
        .build();

    let url_rows_tracker: Rc<RefCell<Vec<gtk::Widget>>> = Rc::new(RefCell::new(Vec::new()));
    let rebuild_urls: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    {
        let contacts = contacts.clone();
        let urls_group = urls_group.clone();
        let url_rows_tracker = url_rows_tracker.clone();
        let on_modified = on_modified.clone();
        let rebuild_urls_ref = rebuild_urls.clone();
        let refresh_banner = refresh_validation_banner.clone();
        let show_validation = show_validation.clone();

        let closure: Rc<dyn Fn()> = Rc::new(move || {
            for w in url_rows_tracker.borrow_mut().drain(..) {
                urls_group.remove(&w);
            }

            let count = contacts.borrow()[index].urls.len();
            if count == 0 {
                let empty = adw::ActionRow::builder()
                    .title("No websites")
                    .use_markup(false)
                    .build();
                urls_group.add(&empty);
                url_rows_tracker.borrow_mut().push(empty.upcast());
                return;
            }

            for u_idx in 0..count {
                let current_url = contacts.borrow()[index].urls[u_idx].clone();
                let row = adw::EntryRow::builder()
                    .title("Website (https://example.com)")
                    .text(&current_url)
                    .build();

                let warn_icon = create_row_warning_icon();
                let del_btn = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .css_classes(["flat", "destructive-action"])
                    .valign(gtk::Align::Center)
                    .tooltip_text("Delete Website")
                    .build();
                row.add_suffix(&warn_icon);
                row.add_suffix(&del_btn);

                let initial_err = if show_validation.get() {
                    validate_url(&current_url)
                } else {
                    None
                };
                apply_row_validation(&row, &warn_icon, initial_err);

                {
                    let contacts = contacts.clone();
                    let on_modified = on_modified.clone();
                    let refresh_banner = refresh_banner.clone();
                    let warn_icon = warn_icon.clone();
                    let show_validation = show_validation.clone();
                    row.connect_changed(move |r| {
                        let text = r.text().to_string();
                        if show_validation.get() {
                            apply_row_validation(r, &warn_icon, validate_url(&text));
                        }
                        contacts.borrow_mut()[index].urls[u_idx] = text;
                        refresh_banner();
                        on_modified();
                    });
                }

                {
                    let contacts = contacts.clone();
                    let on_modified = on_modified.clone();
                    let rebuild_urls_ref = rebuild_urls_ref.clone();
                    let refresh_banner = refresh_banner.clone();
                    del_btn.connect_clicked(move |_| {
                        contacts.borrow_mut()[index].urls.remove(u_idx);
                        refresh_banner();
                        on_modified();
                        if let Some(cb) = rebuild_urls_ref.borrow().as_ref() {
                            cb();
                        }
                    });
                }

                urls_group.add(&row);
                url_rows_tracker.borrow_mut().push(row.upcast());
            }
        });

        closure();
        *rebuild_urls.borrow_mut() = Some(closure);
    }

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let rebuild_urls = rebuild_urls.clone();
        let refresh_banner = refresh_validation_banner.clone();
        add_url_btn.connect_clicked(move |_| {
            contacts.borrow_mut()[index].urls.push(String::new());
            refresh_banner();
            on_modified();
            if let Some(cb) = rebuild_urls.borrow().as_ref() {
                cb();
            }
        });
    }
    page.add(&urls_group);

    // --- Group 5: Addresses ---
    let add_addr_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .label("Add")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();

    let addrs_group = adw::PreferencesGroup::builder()
        .title("Address")
        .header_suffix(&add_addr_btn)
        .build();

    let addr_rows_tracker: Rc<RefCell<Vec<gtk::Widget>>> = Rc::new(RefCell::new(Vec::new()));
    let rebuild_addrs: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    {
        let contacts = contacts.clone();
        let addrs_group = addrs_group.clone();
        let addr_rows_tracker = addr_rows_tracker.clone();
        let on_modified = on_modified.clone();
        let rebuild_addrs_ref = rebuild_addrs.clone();
        let refresh_banner = refresh_validation_banner.clone();
        let show_validation = show_validation.clone();

        let closure: Rc<dyn Fn()> = Rc::new(move || {
            for w in addr_rows_tracker.borrow_mut().drain(..) {
                addrs_group.remove(&w);
            }

            let count = contacts.borrow()[index].addresses.len();
            if count == 0 {
                let empty = adw::ActionRow::builder()
                    .title("No addresses")
                    .use_markup(false)
                    .build();
                addrs_group.add(&empty);
                addr_rows_tracker.borrow_mut().push(empty.upcast());
                return;
            }

            for a_idx in 0..count {
                let addr = contacts.borrow()[index].addresses[a_idx].clone();
                let label = if addr.types.is_empty() {
                    "Address".to_string()
                } else {
                    format!("Address ({})", format_address_type_label(&addr.types))
                };

                let postal_err = validate_postal_code(&addr.postal_code);
                let should_expand = count == 1 || (show_validation.get() && postal_err.is_some());

                let expander = adw::ExpanderRow::builder()
                    .title(&label)
                    .subtitle(&addr.summary())
                    .use_markup(false)
                    .expanded(should_expand)
                    .build();

                let del_btn = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .css_classes(["flat", "destructive-action"])
                    .valign(gtk::Align::Center)
                    .tooltip_text("Delete Address")
                    .build();
                expander.add_suffix(&del_btn);

                let type_row = adw::EntryRow::builder()
                    .title("Type (e.g., WORK, HOME)")
                    .text(&addr.types_string())
                    .build();
                let street_row = adw::EntryRow::builder()
                    .title("Street")
                    .text(&addr.street)
                    .build();
                let city_row = adw::EntryRow::builder()
                    .title("City")
                    .text(&addr.city)
                    .build();
                let region_row = adw::EntryRow::builder()
                    .title("State / Province")
                    .text(&addr.region)
                    .build();
                let postal_row = adw::EntryRow::builder()
                    .title("Postal Code")
                    .text(&addr.postal_code)
                    .build();
                let postal_warn_icon = create_row_warning_icon();
                postal_row.add_suffix(&postal_warn_icon);
                apply_row_validation(
                    &postal_row,
                    &postal_warn_icon,
                    if show_validation.get() {
                        postal_err
                    } else {
                        None
                    },
                );

                let country_row = adw::EntryRow::builder()
                    .title("Country")
                    .text(&addr.country)
                    .build();

                let bind_addr_field =
                    |entry: &adw::EntryRow,
                     warn_opt: Option<gtk::Image>,
                     validator: Option<fn(&str) -> Option<&'static str>>,
                     setter: fn(&mut Address, String)| {
                        let contacts = contacts.clone();
                        let on_modified = on_modified.clone();
                        let refresh_banner = refresh_banner.clone();
                        let show_validation = show_validation.clone();
                        let exp_weak = expander.downgrade();
                        entry.connect_changed(move |r| {
                            let val = r.text().to_string();
                            if show_validation.get() {
                                if let (Some(ref w_icon), Some(val_fn)) = (&warn_opt, validator) {
                                    apply_row_validation(r, w_icon, val_fn(&val));
                                }
                            }
                            let (new_title, new_sub) = {
                                let mut list = contacts.borrow_mut();
                                let a = &mut list[index].addresses[a_idx];
                                setter(a, val);
                                let t = if a.types.is_empty() {
                                    "Address".to_string()
                                } else {
                                    format!("Address ({})", format_address_type_label(&a.types))
                                };
                                (t, a.summary())
                            };
                            if let Some(exp) = exp_weak.upgrade() {
                                exp.set_title(&new_title);
                                exp.set_subtitle(&new_sub);
                            }
                            refresh_banner();
                            on_modified();
                        });
                    };

                bind_addr_field(&type_row, None, None, |a, v| a.set_types_from_str(&v));
                bind_addr_field(&street_row, None, None, |a, v| a.street = v);
                bind_addr_field(&city_row, None, None, |a, v| a.city = v);
                bind_addr_field(&region_row, None, None, |a, v| a.region = v);
                bind_addr_field(
                    &postal_row,
                    Some(postal_warn_icon),
                    Some(validate_postal_code),
                    |a, v| a.postal_code = v,
                );
                bind_addr_field(&country_row, None, None, |a, v| a.country = v);

                expander.add_row(&type_row);
                expander.add_row(&street_row);
                expander.add_row(&city_row);
                expander.add_row(&region_row);
                expander.add_row(&postal_row);
                expander.add_row(&country_row);

                {
                    let contacts = contacts.clone();
                    let on_modified = on_modified.clone();
                    let rebuild_addrs_ref = rebuild_addrs_ref.clone();
                    let refresh_banner = refresh_banner.clone();
                    del_btn.connect_clicked(move |_| {
                        contacts.borrow_mut()[index].addresses.remove(a_idx);
                        refresh_banner();
                        on_modified();
                        if let Some(cb) = rebuild_addrs_ref.borrow().as_ref() {
                            cb();
                        }
                    });
                }

                addrs_group.add(&expander);
                addr_rows_tracker.borrow_mut().push(expander.upcast());
            }
        });

        closure();
        *rebuild_addrs.borrow_mut() = Some(closure);
    }

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        let rebuild_addrs = rebuild_addrs.clone();
        let refresh_banner = refresh_validation_banner.clone();
        add_addr_btn.connect_clicked(move |_| {
            contacts.borrow_mut()[index].addresses.push(Address {
                types: vec!["WORK".to_string()],
                ..Default::default()
            });
            refresh_banner();
            on_modified();
            if let Some(cb) = rebuild_addrs.borrow().as_ref() {
                cb();
            }
        });
    }
    page.add(&addrs_group);

    // --- Group 6: Notes ---
    let note_group = adw::PreferencesGroup::builder().title("Notes").build();

    let note_buffer = gtk::TextBuffer::new(None);
    note_buffer.set_text(&initial_card.note);

    let note_view = gtk::TextView::builder()
        .buffer(&note_buffer)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .height_request(96)
        .build();

    let note_card = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    note_card.append(&note_view);

    {
        let contacts = contacts.clone();
        let on_modified = on_modified.clone();
        note_buffer.connect_changed(move |buf| {
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
            contacts.borrow_mut()[index].note = text.to_string();
            on_modified();
        });
    }

    note_group.add(&note_card);
    page.add(&note_group);

    // --- Group 7: Delete Contact ---
    let danger_group = adw::PreferencesGroup::new();
    let delete_contact_btn = gtk::Button::builder()
        .label("Delete Contact")
        .css_classes(["pill", "destructive-action"])
        .halign(gtk::Align::Center)
        .margin_top(12)
        .margin_bottom(24)
        .build();

    delete_contact_btn.connect_clicked(move |_| {
        on_delete_contact(index);
    });

    danger_group.add(&delete_contact_btn);
    page.add(&danger_group);

    // Triggered when "Done" is clicked: reveals all red highlights & checks validity
    let validate_and_highlight: Rc<dyn Fn() -> bool> = {
        let contacts = contacts.clone();
        let page = page.clone();
        let show_validation = show_validation.clone();
        let refresh_banner = refresh_validation_banner.clone();
        let refresh_identity = refresh_identity_ui.clone();
        let rebuild_phones = rebuild_phones.clone();
        let rebuild_emails = rebuild_emails.clone();
        let rebuild_urls = rebuild_urls.clone();
        let rebuild_addrs = rebuild_addrs.clone();

        Rc::new(move || {
            show_validation.set(true);
            refresh_banner();
            refresh_identity();

            if let Some(cb) = rebuild_phones.borrow().as_ref() {
                cb();
            }
            if let Some(cb) = rebuild_emails.borrow().as_ref() {
                cb();
            }
            if let Some(cb) = rebuild_urls.borrow().as_ref() {
                cb();
            }
            if let Some(cb) = rebuild_addrs.borrow().as_ref() {
                cb();
            }

            let is_valid = contacts.borrow()[index].validation_errors().is_empty();
            if !is_valid {
                page.scroll_to_top();
            }
            is_valid
        })
    };

    ContactEditorView {
        widget: page,
        validate_and_highlight,
    }
}

fn create_row_warning_icon() -> gtk::Image {
    gtk::Image::builder()
        .icon_name("dialog-warning-symbolic")
        .css_classes(["error"])
        .valign(gtk::Align::Center)
        .visible(false)
        .build()
}

fn apply_row_validation(row: &adw::EntryRow, warn_icon: &gtk::Image, error_msg: Option<&str>) {
    match error_msg {
        Some(msg) => {
            row.add_css_class("error");
            warn_icon.set_tooltip_text(Some(msg));
            warn_icon.set_visible(true);
        }
        None => {
            row.remove_css_class("error");
            warn_icon.set_visible(false);
        }
    }
}

fn build_typed_value_row<FType, FVal, FDel>(
    title: &str,
    item: &TypedValue,
    presets: &[&str],
    validator: fn(&str) -> Option<&'static str>,
    show_validation: Rc<Cell<bool>>,
    on_type_changed: FType,
    on_val_changed: FVal,
    on_delete: FDel,
) -> adw::EntryRow
where
    FType: Fn(String) + 'static,
    FVal: Fn(String) + 'static,
    FDel: Fn() + 'static,
{
    let row = adw::EntryRow::builder()
        .title(title)
        .text(&item.value)
        .build();

    let current_type = if item.types.is_empty() {
        presets.first().copied().unwrap_or("CELL").to_string()
    } else {
        item.types_string()
    };

    let mut options: Vec<String> = presets.iter().map(|s| (*s).to_string()).collect();
    if !options.contains(&current_type) {
        options.insert(0, current_type.clone());
    }
    let selected_idx = options.iter().position(|x| x == &current_type).unwrap_or(0) as u32;

    let str_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
    let string_list = gtk::StringList::new(&str_refs);

    let dropdown = gtk::DropDown::builder()
        .model(&string_list)
        .selected(selected_idx)
        .valign(gtk::Align::Center)
        .tooltip_text("Type")
        .build();

    let warn_icon = create_row_warning_icon();

    let del_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .css_classes(["flat", "destructive-action"])
        .valign(gtk::Align::Center)
        .tooltip_text("Delete Row")
        .build();

    row.add_suffix(&warn_icon);
    row.add_suffix(&dropdown);
    row.add_suffix(&del_btn);

    let initial_err = if show_validation.get() {
        validator(&item.value)
    } else {
        None
    };
    apply_row_validation(&row, &warn_icon, initial_err);

    dropdown.connect_selected_notify(move |dd| {
        if let Some(obj) = dd.selected_item() {
            if let Some(str_obj) = obj.downcast_ref::<gtk::StringObject>() {
                on_type_changed(str_obj.string().to_string());
            }
        }
    });

    {
        let warn_icon = warn_icon.clone();
        row.connect_changed(move |r| {
            let text = r.text().to_string();
            if show_validation.get() {
                apply_row_validation(r, &warn_icon, validator(&text));
            }
            on_val_changed(text);
        });
    }

    del_btn.connect_clicked(move |_| {
        on_delete();
    });

    row
}

fn format_role_subtitle(card: &VCard) -> Option<String> {
    match (card.title.trim(), card.org.trim()) {
        ("", "") => None,
        (t, "") => Some(t.to_string()),
        ("", o) => Some(o.to_string()),
        (t, o) => Some(format!("{} • {}", t, o)),
    }
}

fn format_phone_type_label(types: &[String]) -> String {
    let has = |needle: &str| types.iter().any(|t| t.eq_ignore_ascii_case(needle));

    if has("FAX") {
        if has("WORK") {
            return "Work Fax".to_string();
        }
        if has("HOME") {
            return "Home Fax".to_string();
        }
        return "Fax".to_string();
    }
    if has("CELL") || has("MOBILE") {
        return "Mobile".to_string();
    }
    if has("WORK") {
        return "Work".to_string();
    }
    if has("HOME") {
        return "Home".to_string();
    }
    if has("PAGER") {
        return "Pager".to_string();
    }
    if has("CAR") {
        return "Car".to_string();
    }

    let meaningful: Vec<String> = types
        .iter()
        .filter(|t| {
            !matches!(
                t.to_uppercase().as_str(),
                "VOICE" | "PREF" | "MSG" | "BBS" | "MODEM" | "ISDN" | "PCS"
            )
        })
        .map(|t| to_title_case(t))
        .collect();

    if meaningful.is_empty() {
        "Phone".to_string()
    } else {
        meaningful.join(", ")
    }
}

fn format_email_type_label(types: &[String]) -> String {
    let has = |needle: &str| types.iter().any(|t| t.eq_ignore_ascii_case(needle));

    if has("WORK") {
        return "Work".to_string();
    }
    if has("HOME") {
        return "Home".to_string();
    }
    if has("PREF") {
        return "Primary".to_string();
    }

    let meaningful: Vec<String> = types
        .iter()
        .filter(|t| !matches!(t.to_uppercase().as_str(), "INTERNET" | "X400" | "PREF"))
        .map(|t| to_title_case(t))
        .collect();

    if meaningful.is_empty() {
        "Personal".to_string()
    } else {
        meaningful.join(", ")
    }
}

fn format_address_type_label(types: &[String]) -> String {
    let has = |needle: &str| types.iter().any(|t| t.eq_ignore_ascii_case(needle));

    if has("WORK") {
        return "Work".to_string();
    }
    if has("HOME") {
        return "Home".to_string();
    }

    let meaningful: Vec<String> = types
        .iter()
        .filter(|t| {
            !matches!(
                t.to_uppercase().as_str(),
                "DOM" | "INTL" | "POSTAL" | "PARCEL" | "PREF"
            )
        })
        .map(|t| to_title_case(t))
        .collect();

    if meaningful.is_empty() {
        "Address".to_string()
    } else {
        meaningful.join(", ")
    }
}

fn to_title_case(raw: &str) -> String {
    let mut chars = raw.trim().chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

