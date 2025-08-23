pub trait ClipboardSink: Send + Sync {
    fn set_text(&self, text: String) -> Result<(), String>;
}

pub struct SystemClipboard;

impl ClipboardSink for SystemClipboard {
    fn set_text(&self, text: String) -> Result<(), String> {
        let res = arboard::Clipboard::new().and_then(|mut c| c.set_text(text));
        res.map_err(|e| e.to_string())
    }
}