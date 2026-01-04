use std::{
    env,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use toml::{to_string_pretty, Table, Value};

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn append_line(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(file, "{line}").unwrap();
}

fn main() {
    let path = env::args().nth(1).expect("Usage: rpg-generator <path>");
    let raw = fs::read_to_string(path).unwrap();
    let rpg = raw.parse::<Table>().unwrap();

    let internal_path = rpg.get("root_directory").unwrap().as_str().unwrap();
    let output_root = PathBuf::from("dist/content").join(internal_path.trim_start_matches('/'));

    let locations = rpg.get("location").unwrap().as_array().unwrap();
    for location in locations {
        let data = location.as_table().unwrap();
        let name = data.get("name").and_then(Value::as_str).unwrap();
        let location_toml = to_string_pretty(location).unwrap();

        let location_dir = output_root.join("locations").join(name);
        write_file(&location_dir.join("!!location.toml"), &location_toml);
        write_file(&location_dir.join("!!connections.txt"), "");
    }

    let enemies = rpg.get("enemy").unwrap().as_array().unwrap();
    for enemy in enemies {
        let data = enemy.as_table().unwrap();
        let name = data.get("name").and_then(Value::as_str).unwrap();
        let enemy_toml = to_string_pretty(enemy).unwrap();

        let enemy_path = output_root.join("enemies").join(format!("{name}.toml"));
        write_file(&enemy_path, &enemy_toml);
    }

    let player = rpg.get("player").unwrap();
    let player_toml = to_string_pretty(player).unwrap();
    write_file(&output_root.join("!!player.toml"), &player_toml);

    let items = rpg.get("item").unwrap().as_array().unwrap();
    for item in items {
        let name = item
            .as_table()
            .unwrap()
            .get("name")
            .and_then(Value::as_str)
            .unwrap();
        let item_toml = to_string_pretty(item).unwrap();

        let item_path = output_root.join("items").join(format!("{name}.toml"));
        write_file(&item_path, &item_toml);
    }

    let connections = rpg.get("connection").unwrap().as_array().unwrap();
    for connection in connections {
        let pair = connection
            .as_table()
            .unwrap()
            .get("locations")
            .unwrap()
            .as_array()
            .unwrap();

        if let [location_1, location_2] = pair.as_slice() {
            let location_1 = location_1.as_str().unwrap();
            let location_2 = location_2.as_str().unwrap();

            let location_1_path = output_root
                .join("locations")
                .join(location_1)
                .join("!!connections.txt");
            let location_2_path = output_root
                .join("locations")
                .join(location_2)
                .join("!!connections.txt");
            append_line(&location_1_path, location_2);
            append_line(&location_2_path, location_1);
        }
    }
}
