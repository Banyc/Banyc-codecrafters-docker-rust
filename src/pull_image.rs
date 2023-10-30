use std::borrow::Cow;

use async_compression::tokio::bufread::GzipDecoder;
use tokio::io::AsyncWriteExt;

use crate::{
    mounting::{mount_layers, mount_writable_tmp_fs},
    token_auth::pass_token_auth,
    unpack_layer_dir, PACKED_LAYER_DIR,
};

const MEDIA_TYPE_MANIFEST_LIST: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
const MEDIA_TYPE_DISTRIBUTION: &str = "application/vnd.docker.distribution.manifest.v2+json";
const MEDIA_TYPE_OCI: &str = "application/vnd.oci.image.manifest.v1+json";

pub async fn pull(registry: &str, image: &str, container_name: &str) {
    let registry_base = format!("{registry}/v2");
    let (image_name, image_version) = image.split_once(':').unwrap();
    let image_name: Cow<'_, str> = match image_name.contains('/') {
        true => image_name.into(),
        false => format!("library/{}", image_name).into(),
    };
    // https://distribution.github.io/distribution/spec/api/#pulling-an-image-manifest
    let url_manifests = format!("{registry_base}/{image_name}/manifests/{image_version}");

    // https://distribution.github.io/distribution/spec/manifest-v2-2/#manifest-list
    let resp = pass_token_auth(|client| {
        client
            .get(&url_manifests)
            .header("Accept", MEDIA_TYPE_MANIFEST_LIST)
    })
    .await;
    // dbg!(&resp);
    // dbg!(&resp.text().await.unwrap());
    let resp: serde_json::Value = resp.json().await.unwrap();
    let manifest_list: models::ImageManifestList = serde_json::from_value(resp.clone()).unwrap();
    if manifest_list.schema_version() != 2 {
        panic!(
            "Manifest list schema version `{}` not supported",
            manifest_list.schema_version()
        );
    }
    let manifest_list: models::ImageManifestListV2 = serde_json::from_value(resp).unwrap();
    // dbg!(&manifest_list);
    let manifest = &manifest_list
        .manifests()
        .iter()
        .find(|manifest| manifest.platform().architecture() == docker_arch())
        .unwrap();
    let (media_type, digest) = (manifest.media_type(), manifest.digest());

    match media_type.as_str() {
        MEDIA_TYPE_DISTRIBUTION => {
            // https://distribution.github.io/distribution/spec/manifest-v2-2/#image-manifest
            handle_manifest(
                &registry_base,
                &image_name,
                digest,
                MEDIA_TYPE_DISTRIBUTION,
                container_name,
            )
            .await
        }
        // https://github.com/opencontainers/image-spec/blob/main/manifest.md
        MEDIA_TYPE_OCI => {
            handle_manifest(
                &registry_base,
                &image_name,
                digest,
                MEDIA_TYPE_OCI,
                container_name,
            )
            .await
        }
        _ => panic!("{media_type}"),
    }
}

async fn handle_manifest(
    registry_base: &str,
    image_name: &str,
    digest: &str,
    accept: &str,
    container_name: &str,
) {
    let url_manifest = format!("{registry_base}/{image_name}/manifests/{digest}");
    // let url_manifest = format!("{registry_base}/library/{image_name}/manifests/{image_version}");
    let resp = pass_token_auth(|client| client.get(&url_manifest).header("Accept", accept)).await;
    // dbg!(&resp);
    let manifest: models::ImageManifest = resp.json().await.unwrap();
    // dbg!(&manifest);
    // dbg!(&resp.text().await.unwrap());

    let unpack_layer_dir = unpack_layer_dir(container_name);
    let mut lower_dir_string = String::new();
    for (i, layer) in manifest.layers().iter().enumerate() {
        let unpack_dir = unpack_layer_dir.join(format!("layer.{i}"));

        let digest = layer.digest();

        let _ = tokio::fs::remove_dir_all(&unpack_dir).await;
        tokio::fs::create_dir_all(&unpack_dir).await.unwrap();

        let file_path = pull_layer(registry_base, image_name, i, digest).await;
        let tar_gz = tokio::fs::File::options()
            .read(true)
            .open(file_path)
            .await
            .unwrap();
        let tar_gz = tokio::io::BufReader::new(tar_gz);
        let tar = GzipDecoder::new(tar_gz);
        let mut archive = tokio_tar::Archive::new(tar);
        archive.unpack(&unpack_dir).await.unwrap();

        if i != 0 {
            lower_dir_string.push(':');
        }
        lower_dir_string.push_str(unpack_dir.to_str().unwrap());
    }

    mount_writable_tmp_fs(container_name);
    mount_layers(container_name, &lower_dir_string);
}

// https://distribution.github.io/distribution/spec/api/#pulling-a-layer
async fn pull_layer(
    registry_base: &str,
    image_name: &str,
    layer_index: usize,
    digest: &str,
) -> std::path::PathBuf {
    tokio::fs::create_dir_all(PACKED_LAYER_DIR.as_path())
        .await
        .unwrap();
    let (image_name_left, image_name_right) = image_name.split_once('/').unwrap();
    let file_path = PACKED_LAYER_DIR.join(format!(
        "{image_name_left}.{image_name_right}.{layer_index}.{digest}.tar.gz"
    ));
    if file_path.exists() {
        // Use cached layer
        return file_path;
    }

    let url_blob = format!("{registry_base}/{image_name}/blobs/{digest}");
    // dbg!(&url_blob);
    let resp = pass_token_auth(|client| client.get(&url_blob)).await;
    // dbg!(&resp);

    download(resp, &file_path).await;
    file_path
}

async fn download(resp: reqwest::Response, file_path: impl AsRef<std::path::Path>) {
    let bytes = resp.bytes().await.unwrap();

    let mut file = tokio::fs::File::options()
        .create(true)
        .write(true)
        .open(&file_path)
        .await
        .unwrap();
    file.write_all(&bytes).await.unwrap();
}

#[allow(dead_code)]
mod models {
    use getset::{CopyGetters, Getters};
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImageManifestList {
        #[getset(get_copy = "pub")]
        schema_version: usize,
    }

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImageManifestListV2 {
        #[getset(get_copy = "pub")]
        schema_version: usize,
        #[getset(get = "pub")]
        media_type: String,
        #[getset(get = "pub")]
        manifests: Vec<ImagePlatformManifest>,
    }

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagePlatformManifest {
        #[getset(get = "pub")]
        media_type: String,
        #[getset(get_copy = "pub")]
        size: usize,
        #[getset(get = "pub")]
        digest: String,
        #[getset(get = "pub")]
        platform: ImagePlatform,
    }

    #[derive(Debug, Clone, Deserialize, Getters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagePlatform {
        #[getset(get = "pub")]
        architecture: String,
        #[getset(get = "pub")]
        os: String,
        #[getset(get = "pub")]
        #[serde(rename = "os.version")]
        os_version: Option<String>,
        #[getset(get = "pub")]
        #[serde(rename = "os.features")]
        os_features: Option<Vec<String>>,
        #[getset(get = "pub")]
        variant: Option<String>,
        #[getset(get = "pub")]
        features: Option<Vec<String>>,
    }

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImageManifest {
        #[getset(get_copy = "pub")]
        schema_version: usize,
        #[getset(get = "pub")]
        media_type: String,
        #[getset(get = "pub")]
        config: ImageConfig,
        #[getset(get = "pub")]
        layers: Vec<ImageLayer>,
    }

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImageConfig {
        #[getset(get = "pub")]
        media_type: String,
        #[getset(get_copy = "pub")]
        size: usize,
        #[getset(get = "pub")]
        digest: String,
    }

    #[derive(Debug, Clone, Deserialize, Getters, CopyGetters)]
    #[serde(rename_all = "camelCase")]
    pub struct ImageLayer {
        #[getset(get = "pub")]
        media_type: String,
        #[getset(get_copy = "pub")]
        size: usize,
        #[getset(get = "pub")]
        digest: String,
        #[getset(get = "pub")]
        urls: Option<Vec<String>>,
    }
}

fn docker_arch() -> &'static str {
    let arch = std::env::consts::ARCH;
    match arch {
        "x86" => "i386",
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => arch,
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::root_fs_path;

    use super::*;

    const DEFAULT_REGISTRY: &str = "https://registry.hub.docker.com";

    #[tokio::test]
    #[serial]
    async fn test_pull_distribution() {
        let image = "busybox:latest";
        let _ = tokio::fs::remove_dir_all(root_fs_path("test")).await;
        pull(DEFAULT_REGISTRY, image, "test").await;
    }

    #[tokio::test]
    #[serial]
    async fn test_pull_oci() {
        let image = "ubuntu:latest";
        let _ = tokio::fs::remove_dir_all(root_fs_path("test")).await;
        pull(DEFAULT_REGISTRY, image, "test").await;
    }
}
