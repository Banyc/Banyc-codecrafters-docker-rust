use crate::{
    container_dir, execute_command, pid_file_path, process_alive, pull_image::pull, read_pid,
    root_fs_path, write_pid,
};
use anyhow::{Context, Result};
use clap::Args;

const DOCKER_EXPLORER: &str = "/usr/local/bin/docker-explorer";
const DEFAULT_REGISTRY: &str = "https://registry.hub.docker.com";

#[derive(Debug, Args)]
pub struct RunArgs {
    pub image: String,
    pub command: String,
    pub command_args: Vec<String>,
    #[clap(short, long, default_value_t = String::from("default"))]
    pub name: String,
    #[clap(short, long, default_value_t = false)]
    pub force: bool,
    #[clap(short, long, default_value_t = String::from(DEFAULT_REGISTRY))]
    pub registry: String,
}

impl RunArgs {
    // Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
    pub fn run(self) -> Result<()> {
        let image = &self.image;
        let command = &self.command;
        let command = std::path::Path::new(command);
        let command_args = &self.command_args;

        // Set up container
        let container = container_dir(&self.name);
        let pid_file_path = pid_file_path(&self.name);
        let pid = read_pid(&pid_file_path);
        if let Some(pid) = pid {
            if !self.force && process_alive(pid) {
                panic!("Process `{pid}` may still be running. Use `run --force`.");
            }
        }
        let root = root_fs_path(&self.name);
        #[cfg(target_os = "linux")]
        {
            crate::mounting::unmount(&self.name);
        }
        let _ = std::fs::remove_dir_all(&container);
        std::fs::create_dir_all(&container).unwrap();

        // Set up root directory
        std::fs::create_dir_all(&root).unwrap();

        // Lock this container
        write_pid(&pid_file_path);

        // Pull image
        let name = self.name.clone();
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                pull(&self.registry, image, &name).await;
            });

        // Copy command file `docker-explorer` to the root directory
        let docker_explorer = std::path::Path::new(DOCKER_EXPLORER);
        if docker_explorer.exists() {
            let command_file = docker_explorer.strip_prefix("/").with_context(|| {
                format!("command '{}' is not an absolute path", command.display())
            })?;
            let command_file = root.join(command_file);
            std::fs::create_dir_all(command_file.parent().unwrap()).unwrap();
            std::fs::copy(command, &command_file).with_context(|| {
                format!(
                    "failed to copy '{}' to '{}'",
                    command.display(),
                    command_file.display()
                )
            })?;
        }

        // Create `dev/null` in `root`
        let dev = root.join("dev");
        std::fs::create_dir_all(&dev).unwrap();
        let null = dev.join("null");
        std::fs::File::create(null).unwrap();

        #[cfg(target_os = "linux")]
        {
            crate::mounting::mount_root_fs(&self.name);
        }

        // Execute the command
        execute_command(command, command_args, &root)
    }
}
