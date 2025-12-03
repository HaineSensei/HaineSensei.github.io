use std::collections::{HashMap, HashSet};
use crate::filesystem::{FilePath, helpers::fetch_text};

use super::types::{DirPath, Content};

#[derive(Clone)]
pub struct AbyssFileSystem {
    pub files: HashMap<DirPath, Contents>,
    pub dirs: HashMap<DirPath, Directories>,
}

#[derive(Clone)]
pub struct Contents(pub HashMap<String, Content>);

#[derive(Debug, Clone)]
pub struct Directories(pub HashSet<String>);

impl Contents {
    /// Parse !!contents.txt into Contents
    pub fn from_file(text: &str) -> Self {
        Contents(
            text.lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|name| (name.to_string(), Content::ToFetch))
                .collect()
        )
    }

    /// Merge in-memory additions
    pub fn extend(&mut self, other: Contents) {
        self.0.extend(other.0);
    }

    pub fn new() -> Self {
        Contents(HashMap::new())
    }
}

impl Directories {
    /// Parse !!directories.txt into Directories
    pub fn from_file(text: &str) -> Self {
        Directories(
            text.lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|s| s.to_string())
                .collect()
        )
    }

    /// Merge in-memory additions
    pub fn extend(&mut self, other: &Directories) {
        self.0.extend(other.0.clone());
    }

    pub fn new() -> Self {
        Directories(HashSet::new())
    }
}

impl AbyssFileSystem {
    pub fn new() -> Self {
        AbyssFileSystem {
            files: HashMap::new(),
            dirs: HashMap::new(),
        }
    }

    pub async fn get_contents(&self, dirpath: &DirPath) -> Contents {
        let mut fetched = Contents::from_file(
                    &fetch_text(
                        &format!("{}/!!contents.txt", dirpath.to_string())
                    ).await.unwrap()
                );
        let in_memory = match self.files.get(dirpath) {
            Some(x) => x.clone(),
            None => Contents::new()
        };
        fetched.extend(in_memory);
        fetched
    }

    pub async fn get_directories(&self, dirpath: &DirPath) -> Directories {
        let mut fetched = Directories::from_file(
                    &fetch_text(
                        &format!("{}/!!directories.txt", dirpath.to_string())
                    ).await.unwrap()
                );
        
        let in_memory = match self.dirs.get(dirpath) {
            Some(x) => x.clone(),
            None => Directories::new()
        };
        fetched.extend(&in_memory);
        fetched
    }
}
