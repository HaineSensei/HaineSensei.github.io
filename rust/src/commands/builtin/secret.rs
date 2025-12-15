use std::io::{Cursor, Read};

use crate::{commands::{Command, CommandData}, filesystem::{AbyssFileSystem, Content, Contents, DirPath, Directories, FilePath, NextDir, VIRTUAL_FS}};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use zip::ZipArchive;

const EMPTY_SECRET: &str = 
r#"Usage: secret <password>

You found my secret hideout, good luck getting in though."#;

const PASSWORD_CORRECT_MESSAGE: &str = r#"Oh no!
You found my password and now you'll be able to see my secret lair at root!

Good thing I anticipated this and deleted all the things I wouldn't want you to see..."#;

pub struct Secret;
impl CommandData for Secret {
    fn name(&self) -> &str { "secret" }
}
impl Command for Secret {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            EMPTY_SECRET.to_string()
        } else {
            let password = args.join("");
            let zip_bytes = match fetch_secret_zip().await {
                Ok(x) => x,
                Err(_) => return "I failed to fetch the password checker, so I guess my secrets are safe forever!".to_string(),
            };
            let zip_cursor = Cursor::new(&zip_bytes);
            let mut zip_file = ZipArchive::new(zip_cursor).unwrap();
            match get_zip_contents(&mut zip_file, password.as_bytes()) {
                Ok(afs) => {
                    let secret_lair_base = DirPath(vec![NextDir::In("secret_lair".to_string())]);
                    // Add extracted content to /secret_lair/
                    VIRTUAL_FS.with_borrow_mut(|vfs| {
                        for (dir_path, contents) in afs.files {
                            // Prepend /secret_lair/ to the path
                            let secret_lair_path = secret_lair_base.concat(&dir_path,true);

                            // Insert the files into the virtual filesystem
                            vfs.content.insert(secret_lair_path, contents.0);
                        }
                    });

                    PASSWORD_CORRECT_MESSAGE.to_string()
                },
                Err(_) => "You will never find my true secrets!".to_string(),
            }
        }
    }
}

struct PasswordError;

/// gets zip contents or returns PasswordError.
/// zipped folder treated as root.
fn get_zip_contents(zip_file: &mut ZipArchive<Cursor<&Vec<u8>>>, password: &[u8]) -> Result<AbyssFileSystem,PasswordError> {
    let mut out_fs = AbyssFileSystem::new();

    // probably superfluous due to zip of directory being treated at root
    out_fs.dirs.insert(DirPath::root(), Directories::new());
    out_fs.files.insert(DirPath::root(), Contents::new());

    // main construction
    let mut files: Vec<(FilePath,String)> = Vec::new();
    let mut dirs: Vec<DirPath> = Vec::new();
    for idx in 0..zip_file.len() {
        let mut file = zip_file.by_index_decrypt(idx, password).ok().ok_or(PasswordError)?;
        let file_name = file.name();
        if file.is_dir() {
            let mut path = DirPath::parse(file_name, &DirPath::root());
            if path.0.get(0) == Some(&NextDir::In("secret_lair".to_string())) {
                path = DirPath(path.0[1..].to_vec())
            }
            out_fs.dirs.insert(path.clone(),Directories::new());
            out_fs.files.insert(path.clone(),Contents::new());
            dirs.push(path);
        } else {
            let mut path = FilePath::parse(file_name,&DirPath::root());
            if path.file.as_str() == "REDACTED" {
                continue
            }
            if path.dir.0.get(0) == Some(&NextDir::In("secret_lair".to_string())) {
                path.dir = DirPath(path.dir.0[1..].to_vec())
            }
            let mut file_content = String::new();
            file.read_to_string(&mut file_content).ok().ok_or(PasswordError)?;
            files.push((path,file_content));
        }
    }
    for dir in dirs {
        match dir.super_dir() {
            Some(super_dir) => {
                let dirs = out_fs.dirs.get_mut(&super_dir).expect("malformed zip");
                dirs.0.insert(dir.final_component().unwrap().to_string());
            },
            None => {}
        }
    }
    for (path, content) in files {
        let contents = out_fs.files.get_mut(&path.dir).expect("malformed zip");
        contents.0.insert(path.file,Content::InMemory(content));
    }

    Ok(out_fs)
}

async fn fetch_secret_zip() -> Result<Vec<u8>,()> {
    let window = web_sys::window().unwrap();

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = "/secret_lair.zip";

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|_| ())?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| ())?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| ())?;

    if !resp.ok() {
        return Err(())
    }

    // Get response as ArrayBuffer
    let array_buffer = JsFuture::from(resp.array_buffer().map_err(|_| ())?)
        .await
        .map_err(|_| ())?;

    // Convert to Uint8Array and then to Vec<u8>
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();

    Ok(bytes)
}