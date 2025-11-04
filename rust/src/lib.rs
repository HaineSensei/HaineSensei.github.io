use wasm_bindgen::prelude::*;

/// Simple greeting function - example of calling Rust from JS
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello from Rust, {}!", name)
}

/// Process a command - example of how you might handle terminal commands in Rust
#[wasm_bindgen]
pub fn process_command(command: &str) -> String {
    match command.trim() {
        "rust-hello" => "Hello from Rust! This command was processed by WebAssembly.".to_string(),
        "rust-info" => {
            "Rust WebAssembly Info:\n\
             - Compiled with wasm-bindgen\n\
             - Running in your browser\n\
             - Fast and efficient!".to_string()
        }
        _ => format!("Rust doesn't recognize command: {}", command)
    }
}

/// Example: Calculate fibonacci (demonstrating performance)
#[wasm_bindgen]
pub fn fibonacci(n: u32) -> u64 {
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
