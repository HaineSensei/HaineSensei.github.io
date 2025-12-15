use crate::commands::{Command, CommandData};

pub struct Hello;
impl CommandData for Hello {
    fn name(&self) -> &str { "hello" }
}
impl Command for Hello {
    async fn execute(&self, _args: &[&str]) -> String {
        "Hello from Rust! This command was processed by WebAssembly.".to_string()
    }
}

pub struct Info;
impl CommandData for Info {
    fn name(&self) -> &str { "info" }
}
impl Command for Info {
    async fn execute(&self, _args: &[&str]) -> String {
        "Rust WebAssembly Info:\n\
         - Compiled with wasm-bindgen\n\
         - Running in your browser\n\
         - Fast and efficient!".to_string()
    }
}

pub struct Echo;
impl CommandData for Echo {
    fn name(&self) -> &str { "echo" }
}
impl Command for Echo {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            String::new()
        } else {
            args.join(" ")
        }
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

pub struct Fib;
impl CommandData for Fib {
    fn name(&self) -> &str { "fib" }
}
impl Command for Fib {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: fib <number>".to_string();
        }

        match args[0].parse::<u32>() {
            Ok(n) if n <= 93 => {
                let result = fibonacci(n);
                format!("fibonacci({}) = {}", n, result)
            }
            Ok(_) => "Please enter a number between 0 and 93".to_string(),
            Err(_) => "Usage: fib <number>".to_string(),
        }
    }
}
