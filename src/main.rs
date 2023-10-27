use anyhow::{Context, Result};

// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command = std::path::Path::new(command);
    let command_args = &args[4..];

    // Copy command file to a new directory
    let root = std::path::Path::new("/tmp/mydocker");
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let command_file = command
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

    // Create `dev/null` in `root`
    let dev = root.join("dev");
    std::fs::create_dir_all(&dev).unwrap();
    let null = dev.join("null");
    std::fs::File::create(null).unwrap();

    // Chroot the new directory
    std::os::unix::fs::chroot(root).unwrap();
    std::env::set_current_dir("/").unwrap();

    // Execute the command
    let output = std::process::Command::new(command)
        .args(command_args)
        .output()
        .with_context(|| {
            format!(
                "Tried to run '{:?}' with arguments {:?}",
                command, command_args
            )
        })?;

    // Redirect outputs
    let std_out = std::str::from_utf8(&output.stdout)?;
    print!("{std_out}");
    let std_err = std::str::from_utf8(&output.stderr)?;
    eprint!("{std_err}");

    // Return exit code
    if let Some(code) = output.status.code() {
        std::process::exit(code);
    }
    Ok(())
}
