#[cfg(target_os = "linux")]
pub fn mount(root: impl AsRef<std::path::Path>) {
    // Mount `/proc`
    {
        let proc_dir = root.as_ref().join("proc");
        std::fs::create_dir_all(&proc_dir).unwrap();

        nix::mount::mount(
            Some("/proc"),
            &proc_dir,
            Some("proc"),
            nix::mount::MsFlags::empty(),
            None::<&std::path::Path>,
        )
        .unwrap();
    }
}

#[cfg(not(target_os = "linux"))]
pub fn mount(_root: impl AsRef<std::path::Path>) {}

#[cfg(target_os = "linux")]
pub fn unmount(root: impl AsRef<std::path::Path>) {
    // Unmount `/proc` in `root`
    {
        let proc_dir = root.as_ref().join("proc");
        let _ = nix::mount::umount2(&proc_dir, nix::mount::MntFlags::MNT_FORCE);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn unmount(_root: impl AsRef<std::path::Path>) {}
