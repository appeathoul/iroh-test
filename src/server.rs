use std::{path::PathBuf, sync::Arc};

use iroh::{RelayMode, Watcher, protocol::Router};
use iroh_blobs::store::fs::FsStore;

use crate::default_relay_map;

#[derive(Clone, Debug)]
pub struct IrohNet {
    pub router: Router,
    pub gossip: iroh_gossip::net::Gossip,
    pub blobs_store: FsStore,
    pub docs: iroh_docs::protocol::Docs,
}

pub async fn start_server(
    secret_key: iroh::SecretKey,
    iroh_db_path: String,
) -> anyhow::Result<IrohNet> {
    let root = PathBuf::from(iroh_db_path);
    // create endpoint
    let endpoint = iroh::Endpoint::builder()
        .secret_key(secret_key)
        .relay_mode(RelayMode::Custom(default_relay_map()))
        .bind()
        .await?;

    // // ensure relay is initialized
    // endpoint.home_relay().initialized().await;

    // add iroh gossip
    let gossip = iroh_gossip::net::Gossip::builder().spawn(endpoint.clone());

    // add iroh blobs
    let store = FsStore::load(&root).await?;

    let blobs = iroh_blobs::BlobsProtocol::new(&store, None);

    // add iroh docs
    let docs = iroh_docs::protocol::Docs::persistent(root.to_owned())
        .spawn(endpoint.clone(), (*blobs).clone(), gossip.clone())
        .await?;

    // build the protocol router
    let builder = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, Arc::new(gossip.clone()))
        .accept(iroh_blobs::ALPN, blobs)
        .accept(iroh_docs::ALPN, docs.clone());

    let router = builder.spawn();

    let iroh_net = IrohNet {
        router,
        gossip,
        blobs_store: store,
        docs,
    };

    Ok(iroh_net)
}
