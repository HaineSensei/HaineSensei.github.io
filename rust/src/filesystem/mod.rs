use std::cell::RefCell;

pub mod file_paths;
pub mod types;
pub mod virtual_fs;
pub mod helpers;
pub mod abyss;
pub mod cave_of_dice;

pub use types::{Manifest, Content, NextDir, DirPath, FilePath};
pub use virtual_fs::VirtualFilesystem;
pub use abyss::{AbyssFileSystem, Contents, Directories};

// Thread-local storage for current directory and virtual filesystem
thread_local! {
    pub static CURRENT_DIR: RefCell<DirPath> = RefCell::new(DirPath::root());
    pub static VIRTUAL_FS: RefCell<VirtualFilesystem> = RefCell::new(VirtualFilesystem::new());
    pub static ABYSS_FS: RefCell<AbyssFileSystem> = RefCell::new(AbyssFileSystem::new());
    pub static CAVE_OF_DICE_INITIALISED : RefCell<bool> = RefCell::new(false);
}
