use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::TeleponuWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct TeleponuApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for TeleponuApplication {
        const NAME: &'static str = "TeleponuApplication";
        type Type = super::TeleponuApplication;
        type ParentType = adw::Application;
    }

impl ObjectImpl for TeleponuApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("win.add-contact", &["<primary>n"]);
            obj.set_accels_for_action("win.open", &["<primary>o"]);
            obj.set_accels_for_action("win.save", &["<primary>s"]);
            obj.set_accels_for_action("win.save-as", &["<primary><shift>s"]);
            obj.set_accels_for_action("app.preferences", &["<primary>comma"]);
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for TeleponuApplication {
        fn activate(&self) {
            let application = self.obj();
            let window = application.active_window().unwrap_or_else(|| {
                let window = TeleponuWindow::new(&*application);
                window.upcast()
            });

            window.present();
        }
    }

    impl GtkApplicationImpl for TeleponuApplication {}
    impl AdwApplicationImpl for TeleponuApplication {}
}

glib::wrapper! {
    pub struct TeleponuApplication(ObjectSubclass<imp::TeleponuApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl TeleponuApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("resource-base-path", "/dev/labambangpunto/teleponu")
            .build()
    }

    fn setup_gactions(&self) {
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| {
                if let Some(win) = app.active_window() {
                    if let Some(teleponu_win) = win.downcast_ref::<TeleponuWindow>() {
                        teleponu_win.show_preferences();
                    }
                }
            })
            .build();

        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                if let Some(win) = app.active_window() {
                    win.close();
                } else {
                    app.quit();
                }
            })
            .build();

        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();

        self.add_action_entries([preferences_action, quit_action, about_action]);
    }

fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Teleponu")
            .application_icon("dev.labambangpunto.teleponu")
            .developer_name("labambangpunto")
            .version(VERSION)
            .website("https://labambangpunto.github.io/teleponu")
            .comments("vCard (VCF) version 3.0 contact file reader and editor.")
            .developers(vec!["labambangpunto"])
            .copyright("© 2026 labambangpunto")
            .build();

        about.present(Some(&window));
    }
}
