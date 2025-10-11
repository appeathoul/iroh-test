use anyhow::Result;
use std::ops::{Deref, DerefMut};

use crate::{
    iroh_create_author, iroh_create_doc,
    store::{GetProperties, IrohCls, IrohProperties, ToBytes},
};
use iroh_docs::{
    DocTicket,
    api::{
        Doc,
        protocol::{AddrInfoOptions, ShareMode},
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::IrohNet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub blob: Vec<u8>,
}

impl ToBytes<Resource> for Resource {
    fn missing_file(id: String) -> Self {
        Resource {
            id,
            name: "文件不存在".to_string(),
            blob: vec![],
        }
    }
}

pub struct Resources(IrohCls<Resource>);

impl Deref for Resources {
    type Target = IrohCls<Resource>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Resources {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GetProperties for Resources {
    fn get_doc(&self) -> &Doc {
        &self.0.doc
    }
}

impl Resources {
    pub async fn new(ticket: &Option<DocTicket>, node: IrohNet) -> anyhow::Result<Self> {
        let doc = iroh_create_doc(&node, &ticket).await?;

        let author_common = iroh_create_author(&node).await?;
        if !ticket.is_some() {
            let ticket = doc
                .share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses)
                .await?;
            Ok(Resources(IrohCls::<Resource> {
                node,
                doc,
                ticket: Some(ticket),
                author: author_common,
                entity: None,
            }))
        } else {
            Ok(Resources(IrohCls::<Resource> {
                node,
                doc,
                ticket: None,
                author: author_common,
                entity: None,
            }))
        }
    }

    pub async fn add_file(&self, name: String, blob: Vec<u8>) -> Result<()> {
        let file_id = Uuid::new_v4().to_string();
        let resource = Resource {
            id: file_id,
            name,
            blob,
        };

        self.0
            .insert_bytes(resource.id.as_bytes(), resource.as_bytes()?)
            .await
    }
}
