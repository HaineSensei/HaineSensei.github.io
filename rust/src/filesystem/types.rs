use serde::Deserialize;
use super::VIRTUAL_FS;

#[derive(Deserialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
}

#[derive(Deserialize, Clone)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub directories: Vec<String>,
}

/// Content can either be in memory or needs to be fetched
#[derive(Clone)]
pub enum Content {
    InMemory(String),
    ToFetch,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum NextDir {
    In(String),
    Out
}

// DirPath([In("usr"),Out,In("Documents")]) interpreted as /usr/../Documents
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DirPath(pub Vec<NextDir>);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FilePath {
    pub dir: DirPath,
    pub file: String
}

impl FilePath {
    pub fn new(dir: DirPath, file: String) -> Self {
        Self { dir, file }
    }

    // Parse a path string into FilePath
    pub fn parse(path: &str, current_dir: &DirPath) -> Self {
        let parts: Vec<&str> = path.rsplitn(2, '/').collect();

        if parts.len() == 1 {
            // Just a filename, use current directory
            Self::new(current_dir.clone(), parts[0].to_string())
        } else {
            // Has directory component
            let filename = parts[0].to_string();
            let dir_part = parts[1];

            let dir = if path.starts_with('/') {
                // Absolute path
                let mut new_dir = DirPath::root();
                for component in dir_part.split('/').filter(|s| !s.is_empty()) {
                    match component {
                        "." => {},
                        ".." => new_dir.cd(&NextDir::Out, true),
                        name => new_dir.cd(&NextDir::In(name.to_string()), true),
                    }
                }
                new_dir
            } else {
                // Relative path
                let mut new_dir = current_dir.clone();
                for component in dir_part.split('/').filter(|s| !s.is_empty()) {
                    match component {
                        "." => {},
                        ".." => new_dir.cd(&NextDir::Out, true),
                        name => new_dir.cd(&NextDir::In(name.to_string()), true),
                    }
                }
                new_dir
            };

            Self::new(dir, filename)
        }
    }

    // Get full path as string (e.g., "/blog/making_this.md")
    pub fn to_string(&self) -> String {
        let dir_str = self.dir.to_string();
        if dir_str == "/" {
            format!("/{}", self.file)
        } else {
            format!("{}/{}", dir_str, self.file)
        }
    }

    // Get URL for fetching from content directory
    pub fn to_url(&self) -> String {
        let dir_str = self.dir.to_string();
        let path_component = if dir_str == "/" {
            "".to_string()
        } else {
            dir_str.trim_start_matches('/').to_string() + "/"
        };
        format!("./content/{}{}", path_component, self.file)
    }

    // Check if this file exists in the virtual filesystem
    pub fn exists(&self) -> bool {
        VIRTUAL_FS.with(|vfs| {
            vfs.borrow().file_exists(self)
        })
    }
}

impl DirPath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn normalised(&self, at_root: bool) -> Self {
        let mut out = Self::root();
        for v in &self.0 {
            out.cd(v, at_root);
        }
        out
    }

    pub fn normalise(&mut self, at_root: bool) {
        self.0 = self.normalised(at_root).0;
    }

    pub fn cd(&mut self, next: &NextDir, at_root: bool) {
        let DirPath(vec) = self;
        match next {
            NextDir::In(_) => {
                vec.push(next.clone());
            },
            NextDir::Out => {
                if !at_root {
                    match vec.pop() {
                        None => {
                            vec.push(NextDir::Out);
                        },
                        Some(NextDir::Out) => {
                            vec.push(NextDir::Out);
                            vec.push(NextDir::Out);
                        },
                        _ => {}
                    };
                } else {
                    vec.pop();
                }
            },
        }
    }

    pub fn to_string(&self) -> String {
        if self.0.is_empty() {
            return "/".to_string();
        }
        let mut result = String::new();
        for dir in &self.0 {
            match dir {
                NextDir::In(name) => {
                    result.push('/');
                    result.push_str(name);
                },
                NextDir::Out => {
                    result.push_str("/..");
                }
            }
        }
        result
    }

    pub fn concat(&self, relative: &Self, at_root: bool) -> Self {
        let mut out = self.clone();
        for v in &relative.0 {
            out.cd(v,at_root);
        }
        out
    }

    // Parse a path string into DirPath
    pub fn parse(path: &str, current_dir: &DirPath) -> Self {
        // Handle special case for root
        if path == "/" {
            return DirPath::root();
        }

        // Determine starting directory
        let mut new_path = if path.starts_with('/') {
            // Absolute path
            DirPath::root()
        } else {
            // Relative path
            current_dir.clone()
        };

        // Process path components
        for component in path.split('/').filter(|s| !s.is_empty()) {
            match component {
                "." => {}, // Stay in current directory
                ".." => new_path.cd(&NextDir::Out, true),
                name => new_path.cd(&NextDir::In(name.to_string()), true),
            }
        }

        new_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_path() {
        let path = DirPath::root();
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn test_simple_path() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("usr".to_string()), true);
        path.cd(&NextDir::In("local".to_string()), true);
        assert_eq!(path.to_string(), "/usr/local");
    }

    #[test]
    fn test_path_with_out_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("usr".to_string()), true);
        path.cd(&NextDir::In("local".to_string()), true);
        path.cd(&NextDir::Out, true);
        path.cd(&NextDir::In("bin".to_string()), true);
        assert_eq!(path.to_string(), "/usr/bin");
    }

    #[test]
    fn test_path_with_out_not_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("usr".to_string()), false);
        path.cd(&NextDir::In("local".to_string()), false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::In("bin".to_string()), false);
        assert_eq!(path.to_string(), "/usr/bin");
    }

    #[test]
    fn test_normalization_at_root() {
        let path = DirPath(vec![
            NextDir::In("usr".to_string()),
            NextDir::Out,
            NextDir::In("Documents".to_string()),
        ]);
        let normalized = path.normalised(true);
        assert_eq!(normalized.to_string(), "/Documents");
    }

    #[test]
    fn test_normalization_not_at_root() {
        let path = DirPath(vec![
            NextDir::In("usr".to_string()),
            NextDir::Out,
            NextDir::In("Documents".to_string()),
        ]);
        let normalized = path.normalised(false);
        assert_eq!(normalized.to_string(), "/Documents");
    }

    #[test]
    fn test_out_beyond_root_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::Out, true); // Try to go up from root when at_root=true
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn test_out_beyond_root_not_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::Out, false); // Try to go up from root when at_root=false
        assert_eq!(path.to_string(), "/..");
    }

    #[test]
    fn test_complex_path() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("home".to_string()), true);
        path.cd(&NextDir::In("user".to_string()), true);
        path.cd(&NextDir::In("docs".to_string()), true);
        path.cd(&NextDir::Out, true);
        path.cd(&NextDir::Out, true);
        path.cd(&NextDir::In("projects".to_string()), true);
        assert_eq!(path.to_string(), "/home/projects");
    }

    #[test]
    fn test_complex_path_not_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("home".to_string()), false);
        path.cd(&NextDir::In("user".to_string()), false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::In("projects".to_string()), false);
        assert_eq!(path.to_string(), "/../projects");
    }

    #[test]
    fn test_concat_simple() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);
        base.cd(&NextDir::In("user".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::In("documents".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home/user/documents");
    }

    #[test]
    fn test_concat_with_out() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);
        base.cd(&NextDir::In("user".to_string()), true);
        base.cd(&NextDir::In("projects".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("documents".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home/user/documents");
    }

    #[test]
    fn test_concat_multiple_out_at_root() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("usr".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("home".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home");
    }

    #[test]
    fn test_concat_multiple_out_not_at_root() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("usr".to_string()), false);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("home".to_string()), false);

        let result = base.concat(&relative, false);
        assert_eq!(result.to_string(), "/../home");
    }

    #[test]
    fn test_concat_empty_relative() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);

        let relative = DirPath::root();

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home");
    }

    #[test]
    fn test_concat_complex_path() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);
        base.cd(&NextDir::In("user".to_string()), true);
        base.cd(&NextDir::In("projects".to_string()), true);
        base.cd(&NextDir::In("rust".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("documents".to_string()), false);
        relative.cd(&NextDir::In("notes.txt".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home/user/documents/notes.txt");
    }
}
