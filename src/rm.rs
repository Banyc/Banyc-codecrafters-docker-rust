use clap::Args;

use crate::{container_dir, mounting::unmount, root_fs_path};

#[derive(Debug, Args)]
pub struct RmArgs {
    pub containers: Vec<String>,
}

impl RmArgs {
    pub fn run(self) -> anyhow::Result<()> {
        for name in self.containers {
            let container = container_dir(&name);
            let root_fs = root_fs_path(&name);
            unmount(&root_fs);
            let _ = std::fs::remove_dir_all(&container);
        }
        Ok(())
    }
}
