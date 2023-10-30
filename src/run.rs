use crate::{
    container_dir, execute_command,
    mounting::{mount, unmount},
    pid_file_path,
    pull_image::pull,
    read_pid, write_pid,
};
use anyhow::{Context, Result};
use clap::Args;

const DOCKER_EXPLORER: &str = "/usr/local/bin/docker-explorer";

#[derive(Debug, Args)]
pub struct RunArgs {
    pub image: String,
    pub command: String,
    pub command_args: Vec<String>,
    #[clap(short, long, default_value_t = String::from("default"))]
    pub name: String,
    #[clap(short, long, default_value_t = false)]
    pub force: bool,
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
        if !self.force && pid.is_some() {
            let pid = pid.unwrap();
            panic!("Process `{pid}` may still be running. Use `run --force`.");
        }
        let root = container.join("rootfs");
        unmount(&root);
        let _ = std::fs::remove_dir_all(&container);
        std::fs::create_dir_all(&container).unwrap();

        // Set up root directory
        std::fs::create_dir_all(&root).unwrap();

        // Lock this container
        write_pid(&pid_file_path);

        // Pull image
        let root_clone = root.clone();
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                pull(image, root_clone).await;
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

        mount(&root);

        // Execute the command
        execute_command(command, command_args, &root)
    }
}
