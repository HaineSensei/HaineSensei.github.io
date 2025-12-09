use std::{collections::{HashMap, HashSet}, sync::LazyLock};

use crate::filesystem::{ABYSS_FS, AbyssFileSystem, CAVE_OF_DICE_INITIALISED, Content, Contents, DirPath, Directories, NextDir, helpers::path_in_abyss};

use rand::{prelude::*, random, random_range};

const DICE_SIZES : [u8; 6] = [4, 6, 8, 10, 12, 20];

pub fn path_in_cave_of_dice(path: &DirPath) -> bool {
    // note that if you the user decides to create their own cave_of_dice, outside the abyss, then some weird things could happen in memory.
    // I don't think it'll show up for the user besides stopping the main cave_of_dice from working as an abyss directory.
    let v = path.0.iter().position(|x|x == &NextDir::In("cave_of_dice".to_string()));
    if let Some(x) = v {
        initialise_with_file_structure(&DirPath(path.0[..=x].to_vec()), &CAVE_OF_DICE);
        path_in_abyss(path)
    } else {
        false
    }
}

const README: &str =
r#"Hope you like dice, there are a lot of them here."#;

fn path_from_name(name: String) -> DirPath {
    DirPath(vec![NextDir::In(name)])
}

pub static CAVE_OF_DICE : LazyLock<AbyssFileSystem> = LazyLock::new(|| {
    let mut filesystem = AbyssFileSystem::new();

    // depth 0
    let dir_names = DICE_SIZES.iter().map(|n|format!("d{n}")).collect::<HashSet<_>>();
    let dirs = Directories(dir_names.clone());
    filesystem.dirs.insert(DirPath::root(), dirs);

    let mut files = Contents(HashMap::new());
    files.0.insert("README.md".into(),Content::InMemory(README.into()));
    filesystem.files.insert(DirPath::root(), files);

    // remainder setup
    let mut remaining_paths = dir_names
        .into_iter()
        .map(|x| DirPath(vec![NextDir::In(x)]))
        .collect::<Vec<_>>();

    // main build loop
    while let Some(path) = remaining_paths.pop() {
        let depth = path.0.len();
        let n: u8 = match path.final_component() {
            Some("d4") => 4,
            Some("d6") => 6,
            Some("d8") => 8,
            Some("d10") => 10,
            Some("d12") => 12,
            Some("d20") => 20,
            None => unreachable!(),
            _ => {
                match random_range(0u8..6) {
                    0 => 4,
                    1 => 6,
                    2 => 8,
                    3 => 10,
                    4 => 12,
                    _ => 20
                }
            }
        };

        let mut subdirectories = Directories(HashSet::new());
        for i in 1..=n {
            if random_range(0..(n*(depth as u8))) < 3 {
                let name = format!("route_{i}");
                remaining_paths.push(path.concat(&path_from_name(name.clone()), false));
                subdirectories.0.insert(name);
            }
        }
        filesystem.dirs.insert(path.clone(),subdirectories);

        let mut contents = Contents(HashMap::new());
        contents.0.insert(format!("d{n}.txt"), Content::ToFetch);

        
        filesystem.files.insert(path, contents);
        
    }

    filesystem
});

pub fn initialise_with_file_structure(cod_path: &DirPath, cod_fs: &AbyssFileSystem) {
    if !CAVE_OF_DICE_INITIALISED.with_borrow(|x| *x) {
        ABYSS_FS.with_borrow_mut(|afs| {
            for (path, dirs) in &cod_fs.dirs {
                afs.dirs.insert(cod_path.concat(path,false), dirs.clone());
            }
            for (path, contents) in &cod_fs.files {
                afs.files.insert(cod_path.concat(path, false), contents.clone());
            }
        });
        CAVE_OF_DICE_INITIALISED.with_borrow_mut(|x| {*x = true;});
    }
}
