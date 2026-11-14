//! Shared, dependency-free pieces used by both the Explorer extension (DLL)
//! and the converter app: format tables, the context-menu model, the
//! command-line protocol between them, and settings.

pub mod formats;
pub mod menu;
pub mod presets;
pub mod request;
pub mod settings;

pub use formats::{Category, Format};
pub use request::Preset;
pub use settings::Settings;

/// CLSID of the Explorer context-menu handler in `fileflip_shell.dll`.
pub const SHELL_CLSID: &str = "{4A8660F0-8A4E-45E6-86B1-77D553F9148A}";
pub const REPO_URL: &str = "https://github.com/xsobhi/fileflip";
