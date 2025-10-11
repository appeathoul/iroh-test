use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use futures::StreamExt;
use iroh_docs::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;

use iroh_docs::{AuthorId, DocTicket, api::Doc};

use crate::doc_subcribe::EventRemoteSync;
use crate::get_images_directory;
use crate::{
    TableType,
    model::{folder::Folders, node::Nodes, resource::Resources},
    server::IrohNet,
};

const MAX_FILE_SIZE: usize = 150 * 1024 * 1024;

pub trait GetProperties {
    // Get document
    fn get_doc(&self) -> &Doc;
}
pub trait ToBytes<T>
where
    T: Serialize + Clone + for<'a> Deserialize<'a>,
    Self: Serialize,
{
    fn from_bytes(bytes: Bytes) -> anyhow::Result<T> {
        let record = bincode::deserialize(&bytes).context("Invalid json data")?;
        Ok(record)
    }
    fn from_string(str: String) -> anyhow::Result<T> {
        let record = serde_json::from_str(&str).context("Invalid string data")?;
        Ok(record)
    }
    fn as_bytes(&self) -> anyhow::Result<Bytes> {
        let buf = bincode::serialize(self)?;
        println!("{}", buf.len());
        ensure!(buf.len() < MAX_FILE_SIZE, "File size exceeds limit");
        Ok(buf.into())
    }
    fn missing_file(id: String) -> T;
}

#[derive(Debug)]
pub struct IrohCls<Entity> {
    pub node: IrohNet,
    pub doc: Doc,
    pub ticket: Option<DocTicket>,
    pub author: AuthorId,
    pub entity: Option<Entity>,
}

pub struct Pair<T>(IrohCls<T>);

pub trait IrohProperties<Entity>
where
    Entity: ToBytes<Entity> + Serialize + Clone + for<'a> Deserialize<'a> + Send,
{
    fn ticket(&self) -> String;

    fn search(&self) -> impl std::future::Future<Output = Result<Vec<Entity>>>;

    fn insert_bytes(
        &self,
        key: impl AsRef<[u8]>,
        content: Bytes,
    ) -> impl std::future::Future<Output = Result<()>>;

    fn bytes_from_entry(
        &self,
        entry: &Entry,
    ) -> impl std::future::Future<Output = anyhow::Result<Entity>>;
}

impl<Entity> IrohProperties<Entity> for IrohCls<Entity>
where
    Entity: ToBytes<Entity> + Serialize + Clone + for<'a> Deserialize<'a> + Send,
{
    fn ticket(&self) -> String {
        if self.ticket.is_some() {
            self.ticket.clone().unwrap().to_string()
        } else {
            String::new()
        }
    }

    async fn insert_bytes(&self, key: impl AsRef<[u8]>, content: Bytes) -> anyhow::Result<()> {
        self.doc
            .set_bytes(self.author, key.as_ref().to_vec(), content)
            .await?;
        Ok(())
    }

    async fn search(&self) -> Result<Vec<Entity>> {
        let entries = self
            .doc
            .get_many(iroh_docs::store::Query::single_latest_per_key())
            .await?;
        let mut entries = entries.collect::<Vec<Result<Entry>>>().await;
        let mut entries = entries.iter_mut();
        let mut entities = Vec::new();
        while let Some(Ok(entry)) = entries.next() {
            let entity = self.bytes_from_entry(&entry).await?;
            entities.push(entity);
        }
        Ok(entities)
    }

    async fn bytes_from_entry(&self, entry: &Entry) -> anyhow::Result<Entity> {
        // In UTF-8, a character is three bytes. If the bytes are not aligned to multiples of 3,
        // an error will occur here, indicating that the key-value pair has a problem
        let id = String::from_utf8(entry.key().to_owned()).context("invalid key")?;
        match self
            .node
            .blobs_store
            .blobs()
            .get_bytes(entry.content_hash())
            .await
        {
            Ok(b) => Entity::from_bytes(b),
            Err(_) => Ok(Entity::missing_file(id)),
        }
    }
}

type ResourceHandle = Arc<RwLock<Option<Resources>>>;
type FolderHandle = Arc<RwLock<Option<Folders>>>;
type NodeHandle = Arc<RwLock<Option<Nodes>>>;
pub struct StoreState {
    pub resource: ResourceHandle,
    pub resource1: ResourceHandle,
    pub resource2: ResourceHandle,
    pub resource3: ResourceHandle,
    pub folder: FolderHandle,
    pub node: NodeHandle,
    pub ticket_string: String,
}

pub async fn create_files(
    iroh: &IrohNet,
    tickets: Option<HashMap<String, DocTicket>>,
) -> Result<StoreState> {
    let tickets = if let Some(ticket) = tickets {
        ticket
    } else {
        HashMap::new()
    };

    let mut store_state = StoreState {
        resource: Arc::new(RwLock::new(None)),
        resource1: Arc::new(RwLock::new(None)),
        resource2: Arc::new(RwLock::new(None)),
        resource3: Arc::new(RwLock::new(None)),
        folder: Arc::new(RwLock::new(None)),
        node: Arc::new(RwLock::new(None)),
        ticket_string: String::new(),
    };

    // Store a ticket array for client use
    let mut ticket_array = vec![String::new(); 6];

    for table_type in TableType::iter() {
        let doc_ticket = tickets.get(table_type.as_ref()).map(|f| f.clone());
        if table_type.as_ref() == "resource" {
            let resources = Resources::new(&doc_ticket, iroh.clone()).await?;
            let namespace_id = &resources.doc.id();

            println!("Resource namespace ID: {}", namespace_id);

            let ticket_share_str = &resources.ticket();
            subscribe_doc(&resources, String::from("resources")).await?;
            ticket_array[0] = ticket_share_str.clone();

            if doc_ticket.is_none() {
                let images_dir = get_images_directory()?;
                println!("Loading images from directory: {:?}", images_dir);
                load_images_to_resources(&resources, &images_dir).await?;
            }
            store_state.resource = Arc::new(RwLock::new(Some(resources)));
        } else if table_type.as_ref() == "folder" {
            let folders = Folders::new(&doc_ticket, iroh.clone()).await?;
            let namespace_id = &folders.doc.id();
            println!("Folder namespace ID: {}", namespace_id);

            let ticket_share_str = &folders.ticket();
            subscribe_doc(&folders, String::from("folders")).await?;
            ticket_array[1] = ticket_share_str.clone();

            if doc_ticket.is_none() {
                for i in 1..10 {
                    folders.insert_folder(format!("New Folder{}", i)).await?;
                }
            }
            store_state.folder = Arc::new(RwLock::new(Some(folders)));
        } else if table_type.as_ref() == "node" {
            let nodes = Nodes::new(&doc_ticket, iroh.clone()).await?;
            let namespace_id = &nodes.doc.id();
            println!("Node namespace ID: {}", namespace_id);

            let ticket_share_str = &nodes.ticket();
            subscribe_doc(&nodes, String::from("nodes")).await?;
            ticket_array[2] = ticket_share_str.clone();
            store_state.node = Arc::new(RwLock::new(Some(nodes)));
        } else if table_type.as_ref() == "resource1" {
            let resources = Resources::new(&doc_ticket, iroh.clone()).await?;
            let namespace_id = &resources.doc.id();

            println!("Resource1 namespace ID: {}", namespace_id);

            let ticket_share_str = &resources.ticket();
            subscribe_doc(&resources, String::from("resources1")).await?;
            ticket_array[3] = ticket_share_str.clone();

            if doc_ticket.is_none() {
                let images_dir = get_images_directory()?;
                println!("Loading images from directory: {:?}", images_dir);
                load_images_to_resources(&resources, &images_dir).await?;
            }
            store_state.resource1 = Arc::new(RwLock::new(Some(resources)));
        } else if table_type.as_ref() == "resource2" {
            let resources = Resources::new(&doc_ticket, iroh.clone()).await?;
            let namespace_id = &resources.doc.id();

            println!("Resource2 namespace ID: {}", namespace_id);

            let ticket_share_str = &resources.ticket();
            subscribe_doc(&resources, String::from("resources2")).await?;
            ticket_array[4] = ticket_share_str.clone();
            store_state.resource2 = Arc::new(RwLock::new(Some(resources)));
        } else if table_type.as_ref() == "resource3" {
            let resources = Resources::new(&doc_ticket, iroh.clone()).await?;
            let namespace_id = &resources.doc.id();

            println!("Resource3 namespace ID: {}", namespace_id);

            let ticket_share_str = &resources.ticket();
            subscribe_doc(&resources, String::from("resources3")).await?;
            ticket_array[5] = ticket_share_str.clone();
            store_state.resource3 = Arc::new(RwLock::new(Some(resources)));
        }
    }
    store_state.ticket_string = ticket_array.join(" ");
    Ok(store_state)
}

/// Traverse and read files in the images directory, and add them to Resources storage
pub async fn load_images_to_resources(resources: &Resources, images_path: &PathBuf) -> Result<()> {
    if !images_path.exists() {
        return Err(anyhow::anyhow!(
            "Images directory does not exist: {:?}",
            images_path
        ));
    }

    let entries = fs::read_dir(images_path)
        .with_context(|| format!("Failed to read directory: {:?}", images_path))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip directories and hidden files (like .DS_Store)
        if path.is_file() && !path.file_name().unwrap().to_string_lossy().starts_with('.') {
            let file_name = path
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
                .to_string_lossy()
                .to_string();

            // Read file content
            let file_content =
                fs::read(&path).with_context(|| format!("Failed to read file: {:?}", path))?;

            println!("Adding file: {} ({} bytes)", file_name, file_content.len());

            // Call add_file to add to storage
            resources
                .add_file(file_name, file_content)
                .await
                .with_context(|| format!("Failed to add file to resources: {:?}", path))?;
        }
    }

    Ok(())
}

async fn subscribe_doc<'a, T>(table: &T, table_name: String) -> Result<()>
where
    T: GetProperties,
{
    let namespace_id = table.get_doc().id();
    // Listen for document modifications
    let mut events = table.get_doc().subscribe().await?;

    let mut event_remote_sync = EventRemoteSync::new(namespace_id, table_name);
    let events_handle = tokio::spawn(async move {
        while let Some(Ok(event)) = events.next().await {
            event_remote_sync.emit_doc_edit(event).await;
        }
    });
    Ok(())
}
