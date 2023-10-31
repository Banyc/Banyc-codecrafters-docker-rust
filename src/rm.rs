use clap::Args;

use crate::container_dir;

#[derive(Debug, Args)]
pub struct RmArgs {
    pub containers: Vec<String>,
}

impl RmArgs {
    pub fn run(self) -> anyhow::Result<()> {
        for name in self.containers {
            let container = container_dir(&name);
            #[cfg(target_os = "linux")]
            {
                crate::mounting::unmount(&name);
            }
            let _ = std::fs::remove_dir_all(&container);
        }
        Ok(())
    }
}
