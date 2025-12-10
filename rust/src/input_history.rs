use std::cell::RefCell;

/// Input history for arrow key navigation through previously entered commands
pub struct InputHistory {
    inputs: Vec<String>,
    index: usize,
}

impl InputHistory {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            index: 0,
        }
    }

    /// Add a new input to history (skips empty strings)
    pub fn add_input(&mut self, input: String) {
        if !input.is_empty() {
            self.inputs.push(input);
            self.index = self.inputs.len();
        }
    }

    /// Navigate backward in history (arrow up)
    /// Returns the previous input, or None if already at the beginning
    pub fn arrow_up(&mut self) -> Option<String> {
        if self.index > 0 {
            self.index -= 1;
            Some(self.inputs[self.index].clone())
        } else {
            None
        }
    }

    /// Navigate forward in history (arrow down)
    /// Returns the next input, or empty string if at the end (for new input)
    /// Returns None if already at the end
    pub fn arrow_down(&mut self) -> Option<String> {
        if self.index < self.inputs.len() {
            self.index += 1;
            if self.index < self.inputs.len() {
                Some(self.inputs[self.index].clone())
            } else {
                // At the end - return empty string for new input
                Some(String::new())
            }
        } else {
            None
        }
    }
}

thread_local! {
    pub static INPUT_HISTORY: RefCell<InputHistory> = RefCell::new(InputHistory::new());
}
