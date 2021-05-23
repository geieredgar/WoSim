use tokio::process::Command;

pub trait EnvExt {
    fn setup_env(&mut self) -> &mut Self;
}

impl EnvExt for Command {
    #[cfg(not(target_os = "macos"))]
    fn setup_env(&mut self) -> &mut Self {
        self
    }

    #[cfg(target_os = "macos")]
    fn setup_env(&mut self) -> &mut Self {
        self.env("MVK_CONFIG_FULL_IMAGE_VIEW_SWIZZLE", "1")
    }
}
