use std::io::{Read, Write};

use crate::{
    mounting::{mount, unmount},
    pull_image::pull,
};
use anyhow::{Context, Result};
use clap::Args;

const DOCKER_EXPLORER: &str = "/usr/local/bin/docker-explorer";
const CONTAINERS: &str = "/tmp/mydocker/containers";

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
        let containers = std::path::Path::new(CONTAINERS);
        let container = containers.join(&self.name);
        let pid_file_path = container.join("pid");
        if !self.force && pid_file_path.exists() {
            let file = std::fs::File::options()
                .read(true)
                .open(&pid_file_path)
                .unwrap();
            let mut file = std::io::BufReader::new(file);
            let mut pid = String::new();
            file.read_to_string(&mut pid).unwrap();
            let pid: usize = pid.parse().unwrap();
            panic!("Process `{pid}` may still be running. Use `run --force`.");
        }
        let root = container.join("rootfs");
        unmount(&root);
        let _ = std::fs::remove_dir_all(&container);
        std::fs::create_dir_all(&container).unwrap();

        // Set up root directory
        std::fs::create_dir_all(&root).unwrap();

        // Lock this container
        {
            let pid = std::process::id();
            let mut file = std::fs::File::options()
                .create(true)
                .write(true)
                .open(&pid_file_path)
                .unwrap();
            file.write_all(format!("{pid}").as_bytes()).unwrap();
        }

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

        // Chroot the root directory
        std::os::unix::fs::chroot(root).unwrap();
        std::env::set_current_dir("/").unwrap();

        // The calling process is not moved into the new namespace.
        // The first child created by the calling process will have the process ID 1 and will assume the role of init(1) in the new namespace.
        #[cfg(target_os = "linux")]
        {
            let res = unsafe { libc::unshare(libc::CLONE_NEWPID) };
            if res != 0 {
                std::process::exit(res);
            }
        }

        // Execute the command
        let mut child = std::process::Command::new(command)
            .args(command_args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "Tried to run '{:?}' with arguments {:?}",
                    command, command_args
                )
            })?;

        // Wait for the child to exit
        let exit_status = child.wait().unwrap();

        // Return exit code
        if let Some(code) = exit_status.code() {
            std::process::exit(code);
        }
        Ok(())
    }
}
