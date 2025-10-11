use std::ops::{Deref, DerefMut};

use iroh_docs::{
    DocTicket,
    api::{
        Doc,
        protocol::{AddrInfoOptions, ShareMode},
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    iroh_create_author, iroh_create_doc,
    server::IrohNet,
    store::{GetProperties, IrohCls, IrohProperties, ToBytes},
};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Folder {
    pub folder_id: String,
    pub folder_name: String,
}

impl ToBytes<Folder> for Folder {
    fn missing_file(id: String) -> Self {
        Folder {
            folder_id: id,
            folder_name: "Untitled".to_string(),
        }
    }
}

pub struct Folders(IrohCls<Folder>);

impl Deref for Folders {
    type Target = IrohCls<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Folders {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl GetProperties for Folders {
    fn get_doc(&self) -> &Doc {
        &self.0.doc
    }
}

impl Folders {
    pub async fn new(ticket: &Option<DocTicket>, node: IrohNet) -> anyhow::Result<Self> {
        let doc = iroh_create_doc(&node, &ticket).await?;

        let author_common = iroh_create_author(&node).await?;
        if !ticket.is_some() {
            let ticket = doc
                .share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses)
                .await?;
            Ok(Folders(IrohCls::<Folder> {
                node,
                doc,
                ticket: Some(ticket),
                author: author_common,
                entity: None,
            }))
        } else {
            Ok(Folders(IrohCls::<Folder> {
                node,
                doc,
                ticket: None,
                author: author_common,
                entity: None,
            }))
        }
    }

    pub async fn insert_folder(&self, folder_name: String) -> anyhow::Result<()> {
        let folder_id = Uuid::new_v4().to_string();
        let folder = Folder {
            folder_id,
            folder_name,
        };

        self.0
            .insert_bytes(folder.folder_id.as_bytes(), folder.as_bytes()?)
            .await
    }
}
