use ini::Ini;
use std::path::Path;

pub fn create_config() {
    if !Path::new("./conf.ini").exists() {
        let mut conf = Ini::new();
        conf.with_section(Some("User"))
            .set("name", "");
        // conf.with_section(Some("Window"))
        //     .set("width", "400.0");
        // conf.with_section(Some("Window"))
        //     .set("height", "500.0");
        conf.write_to_file("conf.ini").unwrap();
    }
}