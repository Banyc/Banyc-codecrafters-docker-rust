use clap::Args;

use crate::{container_dir, mounting::unmount};

#[derive(Debug, Args)]
pub struct RmArgs {
    pub containers: Vec<String>,
}

impl RmArgs {
    pub fn run(self) -> anyhow::Result<()> {
        for name in self.containers {
            let container = container_dir(&name);
            unmount(&name);
            let _ = std::fs::remove_dir_all(&container);
        }
        Ok(())
    }
}
