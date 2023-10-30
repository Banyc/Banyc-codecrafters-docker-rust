use clap::Args;

use crate::{CONTAINERS, PACKED_LAYER_DIR};

#[derive(Debug, Args)]
pub struct LsArgs {}

impl LsArgs {
    pub fn run(self) -> anyhow::Result<()> {
        let containers = &CONTAINERS;
        let containers = std::fs::read_dir(containers.as_path()).unwrap();
        println!("Containers:");
        for container in containers {
            let container = container.unwrap();
            println!("{}", container.file_name().to_str().unwrap());
        }

        let layers = &PACKED_LAYER_DIR;
        let mut images: Vec<String> = vec![];
        let layers = std::fs::read_dir(layers.as_path()).unwrap();
        for layer in layers {
            let layer = layer.unwrap();
            let layer = layer.file_name();
            let mut split = layer.to_str().unwrap().split('.');
            let image_left = split.next().unwrap().to_string();
            let image_right = split.next().unwrap().to_string();
            let image = format!("{image_left}/{image_right}");
            if !images.contains(&image) {
                images.push(image);
            }
        }
        println!("Images:");
        for image in images {
            println!("{}", image);
        }

        Ok(())
    }
}
