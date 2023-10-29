use anyhow::{Context, Result};
use docker_starter_rust::pull_image::pull;

const DOCKER_EXPLORER: &str = "/usr/local/bin/docker-explorer";

// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let image = &args[2];
    let command = &args[3];
    let command = std::path::Path::new(command);
    let command_args = &args[4..];

    // Set up root directory
    let root = std::path::Path::new("/tmp/mydocker/container");
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();

    // Pull image
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            pull(image, &root).await;
        });

    // Copy command file `docker-explorer` to the root directory
    let docker_explorer = std::path::Path::new(DOCKER_EXPLORER);
    if docker_explorer.exists() {
        let command_file = docker_explorer
            .strip_prefix("/")
            .with_context(|| format!("command '{}' is not an absolute path", command.display()))?;
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

    // Chroot the root directory
    std::os::unix::fs::chroot(root).unwrap();
    std::env::set_current_dir("/").unwrap();

    // The calling process is not moved into the new namespace.
    // The first child created by the calling process will have the process ID 1 and will assume the role of init(1) in the new namespace.
    let res = unsafe { libc::unshare(libc::CLONE_NEWPID) };
    if res != 0 {
        std::process::exit(res);
    }

    // Execute the command
    let exit_status = std::process::Command::new(command)
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
        })?
        .wait()
        .unwrap();

    // Return exit code
    if let Some(code) = exit_status.code() {
        std::process::exit(code);
    }
    Ok(())
}
