use std::env;

use toml::{Table, Value, to_string_pretty};

fn main() {
    let path = env::args().nth(1).unwrap();
    let raw = std::fs::read_to_string(path).unwrap();
    let rpg = raw.parse::<Table>().unwrap();
    
    // site path setup:
    let internal_path = rpg.get("root_directory").unwrap().as_str().unwrap();

    // locations:
    let locations = rpg.get("location").unwrap().as_array().unwrap();
    for location in locations {
        let data = location.as_table().unwrap();
        let name = data.get("name").unwrap();
        let location_toml = to_string_pretty(location).unwrap();
        
        // write location_toml to dist/content/<internal_path>/locations/<name>/!!location.toml
        
        // write empty file to dist/content/<internal_path>/locations/<name>/!!connections.txt
    }

    let enemies = rpg.get("enemies").unwrap().as_array().unwrap();
    for enemy in enemies {
        let data = enemy.as_table().unwrap();
        let name = data.get("name").unwrap();
        let enemy_toml = to_string_pretty(enemy).unwrap();

        // write enemy_toml to dist/content/<internal_path>/enemies/<name>.toml

    }

    let player = rpg.get("player").unwrap();
    let player_toml = to_string_pretty(player).unwrap();

    // write player_toml to dist/content/<internal_path>/!!player.toml
    
    let items = rpg.get("items").unwrap().as_array().unwrap();
    for item in items {
        let name = item.as_table().unwrap().get("name").unwrap();
        let item_toml = to_string_pretty(item).unwrap();

        // write item_toml to dist/content/<internal_path>/items/<name>.toml
    }

    let connections = rpg.get("connection").unwrap().as_array().unwrap();
    for connection in connections {
        let pair = connection.as_table().unwrap().get("locations").unwrap().as_array().unwrap();
        if let [location_1,location_2] = pair.as_slice() {

            // add location_1 to dist/content/<internal_path>/locations/<location_2>/!!connections.txt as a new line.
            // do the same for location_2 into location_1.

        }
    }

}

