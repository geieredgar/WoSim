#[derive(Debug)]
pub enum Error {
    NewClipboardFailed(Box<dyn std::error::Error + Send + Sync + 'static>),
    SetClipboardContents(Box<dyn std::error::Error + Send + Sync + 'static>),
}
