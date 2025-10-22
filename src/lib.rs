use std::path::PathBuf;

use anyhow::Result;
use futures::TryStreamExt;
use iroh::{RelayConfig, RelayMap, SecretKey};
use iroh_docs::api::Doc;
use iroh_docs::{Author, AuthorId, DocTicket};
use iroh_relay::RelayQuicConfig;
use url::Url;

use crate::server::IrohNet;

pub mod doc_subcribe;
pub mod model;
pub mod server;
pub mod store;

pub const DEFAULT_RELAY_HOSTNAME: &str = "picorca.com";

pub const AUTHOR: &[u8; 32] = &[
    7, 57, 234, 237, 239, 151, 201, 39, 210, 244, 128, 178, 34, 67, 38, 216, 247, 76, 126, 49, 255,
    112, 41, 183, 79, 0, 138, 66, 249, 34, 109, 14,
];

/// Get the default [`RelayMap`]
pub fn default_relay_map() -> RelayMap {
    RelayMap::from_iter([default_relay_node()])
}

/// Get the default [`RelayNode`]
pub fn default_relay_node() -> RelayConfig {
    // The default CH relay server run by number0.
    let url: Url = format!("https://{DEFAULT_RELAY_HOSTNAME}.:4430")
        .parse()
        .expect("default url");
    RelayConfig {
        url: url.into(),
        quic: Some(RelayQuicConfig::default()),
    }
}

/// Generate a new random private key
pub fn generate_private_key() -> SecretKey {
    SecretKey::generate(&mut rand::rng())
}

#[derive(strum::EnumIter, strum::AsRefStr)]
pub enum TableType {
    #[strum(serialize = "folder")]
    Folder,
    #[strum(serialize = "node")]
    Node,
    #[strum(serialize = "resource")]
    Resource,
    #[strum(serialize = "resource1")]
    Resource1,
    #[strum(serialize = "resource2")]
    Resource2,
    #[strum(serialize = "resource3")]
    Resource3,
}

pub async fn iroh_create_doc(node: &IrohNet, ticket: &Option<DocTicket>) -> Result<Doc> {
    let doc: Doc = match ticket {
        Some(tic) => {
            let doc = node.docs.import(tic.clone()).await?;
            println!("Imported doc with id: {}, ticket: {:?}", doc.id(), tic);
            // doc.start_sync(tic.nodes.clone()).await?;
            doc
        }
        None => {
            let doc = node.docs.create().await?;
            println!("Created new doc with id: {}", doc.id());
            doc
        }
    };
    Ok(doc)
}

pub async fn iroh_create_author(node: &IrohNet) -> Result<AuthorId> {
    let author_list: Vec<_> = node.docs.author_list().await?.try_collect().await?;
    let author = Author::from_bytes(AUTHOR);
    if let Some(_author) = author_list.iter().find(|a| a.as_bytes() == AUTHOR) {
        // todo
    } else {
        let author = Author::from_bytes(AUTHOR);
        node.docs.author_import(author.clone()).await?;
    }
    Ok(author.id())
}

pub fn get_images_directory() -> Result<PathBuf> {
    // Get the path of the current executable file
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not get executable directory"))?;

    // In development environment, images directory is in the project root
    // After packaging, images directory should be in the same directory as the executable
    let images_path = exe_dir.join("images");

    // If images is not found in the executable directory, try to find it in the project root
    if !images_path.exists() {
        // Try to find the project root directory (containing Cargo.toml) by going upward
        let mut current = exe_dir;
        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                let project_images = parent.join("images");
                if project_images.exists() {
                    return Ok(project_images);
                }
            }
            current = parent;
        }
    }

    Ok(images_path)
}
