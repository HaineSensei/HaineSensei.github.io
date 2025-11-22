use wasm_bindgen::prelude::*;

// External JavaScript functions that Rust can call
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = addOutput)]
    pub fn add_output(text: &str);

    #[wasm_bindgen(js_name = clearOutput)]
    pub fn clear_output();

    #[wasm_bindgen(js_name = promptFilePicker)]
    pub fn prompt_file_picker(accept: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = triggerDownload)]
    pub fn trigger_download(content: &[u8], mime_type: &str, filename: &str);

    #[wasm_bindgen(js_name = scrollToBottom)]
    pub fn scroll_to_bottom();
}
