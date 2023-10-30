use std::borrow::Cow;

use clap::Args;

use crate::PACKED_LAYER_DIR;

#[derive(Debug, Args)]
pub struct RmiArgs {
    images: Vec<String>,
}

impl RmiArgs {
    pub fn run(self) -> anyhow::Result<()> {
        for image in self.images {
            let (image_left, image_right): (Cow<str>, Cow<str>) = match image.contains('/') {
                true => image
                    .split_once('/')
                    .map(|(l, r)| (l.into(), r.into()))
                    .unwrap(),
                false => ("library".into(), image.into()),
            };

            let layers = &PACKED_LAYER_DIR;
            let layers = std::fs::read_dir(layers.as_path()).unwrap();
            let mut layers_to_remove = vec![];
            for layer in layers {
                let layer = layer.unwrap();
                let layer_name = layer.file_name();
                let mut split = layer_name.to_str().unwrap().split('.');
                let image_left_ = split.next().unwrap().to_string();
                let image_right_ = split.next().unwrap().to_string();
                if image_left == image_left_ && image_right == image_right_ {
                    layers_to_remove.push(layer.path());
                }
            }
            for layer in layers_to_remove {
                std::fs::remove_file(layer).unwrap();
            }
        }
        Ok(())
    }
}
