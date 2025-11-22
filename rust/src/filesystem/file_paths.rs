use std::sync::LazyLock;

use crate::filesystem::{DirPath, FilePath, NextDir};

static MANUAL_DIR_BASE_PATH: LazyLock<DirPath> = LazyLock::new(|| {
    let mut vec = Vec::new();
    vec.push(NextDir::In("manuals".into()));
    DirPath(vec)
});

pub static VERBOSE_MANUAL_DIR_PATH: LazyLock<DirPath> = LazyLock::new(|| {
    let DirPath(mut vec) = MANUAL_DIR_BASE_PATH.clone();
    vec.push(NextDir::In("verbose".into()));
    DirPath(vec)
});

pub static SIMPLE_MANUAL_DIR_PATH: LazyLock<DirPath> = LazyLock::new(|| {
    let DirPath(mut vec) = MANUAL_DIR_BASE_PATH.clone();
    vec.push(NextDir::In("simple".into()));
    DirPath(vec)
});

pub static HELP_FILE_PATH: LazyLock<FilePath> = LazyLock::new(|| 
    FilePath {
        dir: DirPath(Vec::new()), 
        file: "help.txt".into() 
    }
);

pub static HELP_VERBOSE_FILE_PATH: LazyLock<FilePath> = LazyLock::new(||
    FilePath {
        dir: DirPath(Vec::new()), 
        file: "help-verbose.txt".into() 
    }
);
