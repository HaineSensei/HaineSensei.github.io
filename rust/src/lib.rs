use wasm_bindgen::prelude::*;

/// Main command processor - handles all non-content commands
/// Add new commands here!
#[wasm_bindgen]
pub fn process_command(command: &str) -> String {
    let parts: Vec<&str> = command.trim().split_whitespace().collect();

    if parts.is_empty() {
        return String::new();
    }

    match parts[0] {
        "hello" => {
            "Hello from Rust! This command was processed by WebAssembly.".to_string()
        }

        "info" => {
            "Rust WebAssembly Info:\n\
             - Compiled with wasm-bindgen\n\
             - Running in your browser\n\
             - Fast and efficient!".to_string()
        }

        "fib" => {
            if parts.len() < 2 {
                return "Usage: fib <number>".to_string();
            }

            match parts[1].parse::<u32>() {
                Ok(n) if n <= 93 => {
                    let result = fibonacci(n);
                    format!("fibonacci({}) = {}", n, result)
                }
                Ok(_) => "Please enter a number between 0 and 93".to_string(),
                Err(_) => "Usage: fib <number>".to_string(),
            }
        }

        // Add more commands here!

        _ => format!("Command not found: {}\nType 'help' for available commands.", command)
    }
}

/// Calculate fibonacci number (helper function)
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0u64;
            let mut b = 1u64;
            for _ in 2..=n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}
