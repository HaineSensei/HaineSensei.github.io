use crate::{commands::{Command, CommandData, command_data}, filesystem::{DirPath, FilePath, file_paths::{HELP_FILE_PATH, HELP_VERBOSE_FILE_PATH, SIMPLE_MANUAL_DIR_PATH, VERBOSE_MANUAL_DIR_PATH}, helpers::get_file_content}};

pub struct Help;

impl CommandData for Help {
    fn name(&self) -> &str { "help" }
}

impl Command for Help {

    async fn execute(&self, args: &[&str]) -> String {
        // Silly help handling
        let mut special = true;
        let mut help_count = 0;
        let mut verbose = false;
        args.iter().map(|&arg| {
            match arg {
                "help" => {help_count += 1},
                "-v" => {verbose = true},
                _ => {special = false}
            }
        }).for_each(drop);
        if help_count >= 2 && special {
            let mut out = String::new();
            for _ in 0..help_count {
                let multiplier = if verbose {24} else {8};
                for _ in 0..multiplier {
                    out.push_str("A\u{200B}");
                }
            }
            return out.trim().to_string()
        }

        // Main handling

        // Check for -v flag
        let mut verbose = false;
        let args = args.iter().filter(|&&x|{
            if x == "-v" {
                verbose = true;
                false
            } else if x.is_empty() {
                false
            } else {
                true
            }
        }).collect::<Vec<_>>();
        let mut filepath = match args.get(0) {
            Some(&&command) => {
                command_data(command).manual(verbose)
            },
            None => {
                if verbose {
                    HELP_VERBOSE_FILE_PATH.clone()
                } else {
                    HELP_FILE_PATH.clone()
                }
            }
        };
        match get_file_content(&filepath).await {
            Ok(content) => content,
            Err(_) => {
                // If could not find verbose, try simple instead.
                if filepath.dir == *VERBOSE_MANUAL_DIR_PATH {
                    filepath.dir = SIMPLE_MANUAL_DIR_PATH.clone();
                    match get_file_content(&filepath).await {
                        Ok(content) => return content,
                        Err(_) => {}
                    }
                }
                "Could not find relevant help page".to_string()
            }
        }
    }
}

pub struct About;

impl CommandData for About {
    fn name(&self) -> &str { "about" }
}

impl Command for About {
    async fn execute(&self, _args: &[&str]) -> String {
        let filepath = FilePath::new(DirPath::root(), "about.txt".to_string());
        match get_file_content(&filepath).await {
            Ok(content) => content,
            Err(e) => format!("Error loading about.txt: {}", e),
        }
    }
}

pub struct Contact;

impl CommandData for Contact {
    fn name(&self) -> &str { "contact" }
}

impl Command for Contact {
    async fn execute(&self, _args: &[&str]) -> String {
        let filepath = FilePath::new(DirPath::root(), "contact.txt".to_string());
        match get_file_content(&filepath).await {
            Ok(content) => content,
            Err(e) => format!("Error loading contact.txt: {}", e),
        }
    }
}
