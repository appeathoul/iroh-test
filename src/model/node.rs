use std::ops::{Deref, DerefMut};

use iroh_docs::{
    DocTicket,
    api::{
        Doc,
        protocol::{AddrInfoOptions, ShareMode},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    iroh_create_author, iroh_create_doc,
    server::IrohNet,
    store::{GetProperties, IrohCls, ToBytes},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub node_name: String,
    pub key: i64,
    pub node_id: String,
}

impl ToBytes<Node> for Node {
    fn missing_file(id: String) -> Self {
        Node {
            node_name: "文件不存在".to_string(),
            key: 0,
            node_id: id,
        }
    }
}

pub struct Nodes(IrohCls<Node>);

impl Deref for Nodes {
    type Target = IrohCls<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Nodes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GetProperties for Nodes {
    fn get_doc(&self) -> &Doc {
        &self.0.doc
    }
}

impl Nodes {
    pub async fn new(ticket: &Option<DocTicket>, node: IrohNet) -> anyhow::Result<Self> {
        let doc = iroh_create_doc(&node, &ticket).await?;

        let author_common = iroh_create_author(&node).await?;
        if !ticket.is_some() {
            let ticket = doc
                .share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses)
                .await?;
            Ok(Nodes(IrohCls::<Node> {
                node,
                doc,
                ticket: Some(ticket),
                author: author_common,
                entity: None,
            }))
        } else {
            Ok(Nodes(IrohCls::<Node> {
                node,
                doc,
                ticket: None,
                author: author_common,
                entity: None,
            }))
        }
    }
}
