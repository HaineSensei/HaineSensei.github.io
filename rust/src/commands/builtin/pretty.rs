use crate::commands::{Command, CommandData};
use crate::filesystem::{CURRENT_DIR, FilePath};
use crate::js_interop::add_output;

// Helper to open pretty page in new tab
pub fn open_pretty_page(file_path: &str, path_arg: &str) -> String {
    let url = format!("./pretty.html?content={}", file_path);

    if let Some(window) = web_sys::window() {
        match window.open_with_url_and_target(&url, "_blank") {
            Ok(_) => format!("Opening {} in new tab...", path_arg),
            Err(_) => "Error: Failed to open new tab. Please check your browser's popup settings.".to_string()
        }
    } else {
        "Error: Could not access window object".to_string()
    }
}

pub struct Pretty;
impl CommandData for Pretty {
    fn name(&self) -> &str { "pretty" }
}
impl Command for Pretty {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: pretty <filename>".to_string();
        }

        let path_arg = args[0];
        let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

        // Check if file exists
        if !filepath.exists() {
            return format!("pretty: {}: No such file", path_arg);
        }

        // Check if it's a .md file
        let is_markdown = filepath.file.ends_with(".md");
        let is_html = filepath.file.ends_with(".html");

        if is_markdown || is_html {
            // Open directly
            open_pretty_page(&filepath.to_string(), path_arg)
        } else {
            // Ask for confirmation - set handler for next input
            add_output(&format!("Warning: '{}' is not a markdown or html file. Render anyway? (y/n)", path_arg));

            crate::NEXT_INPUT_HANDLER.with(|h| {
                *h.borrow_mut() = crate::NextInputHandler::PrettyConfirm {
                    filepath: filepath.to_string(),
                    path_arg: path_arg.to_string(),
                };
            });

            String::new()  // No additional output, prompt already displayed
        }
    }
}
