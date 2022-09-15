mod libs;
use libs::{gui as gui, config as cfg};

fn main() {
    cfg::create_config();
    gui::create_gui();
}