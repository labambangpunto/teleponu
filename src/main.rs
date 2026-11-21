mod application;
mod config;
mod vcard;
mod widgets;
mod window;

use self::application::TeleponuApplication;
use self::window::TeleponuWindow;

use config::{GETTEXT_PACKAGE, LOCALEDIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::prelude::*;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let app = TeleponuApplication::new(
        "dev.labambangpunto.teleponu",
        &gio::ApplicationFlags::empty(),
    );

    app.run()
}
