use std::{
    collections::HashMap,
    str::from_utf8,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use iroh_docs::{ContentStatus, NamespaceId, engine::LiveEvent};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteUpdateData {
    // data id
    pub key: String,
    // data size
    pub size: u64,
    // table name
    pub table_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ProgressType {
    // initialization
    OnInit,
    // load all tables
    OnLoadedTable,
    // download all data
    OnLoadedBlob,
}

// data sync event object
#[derive(Debug)]
pub struct EventRemoteSync {
    pub hashmap: Arc<Mutex<HashMap<String, RemoteUpdateData>>>,
    // total number of pending downloads
    pub remaining_remote_num: Arc<AtomicU64>,
    // remaining number of pending downloads in current queue
    pub queue_remote_num: Arc<AtomicU64>,
    // total size of pending downloads
    pub remaining_remote_bytes: Arc<AtomicU64>,
    // current pending download data size
    pub queue_remote_bytes: Arc<AtomicU64>,
    // doc namespace_id
    pub namespace_id: NamespaceId,
    // doc name
    pub table_name: String,
    // whether initialization of fetching table content succeeded
    pub init_successed: Arc<AtomicBool>,
    // whether initialization of fetching files succeeded
    pub init_blob_successed: Arc<AtomicBool>,
    // tx
    pub tx: Arc<mpsc::Sender<String>>,
    // handle
    pub handle: Option<JoinHandle<()>>,
}

impl EventRemoteSync {
    pub fn new(namespace_id: NamespaceId, table_name: String) -> Self {
        let hashmap = HashMap::<String, RemoteUpdateData>::new();

        let hashmap_clone = Arc::new(Mutex::new(hashmap));
        // construct message receiver
        let (tx, mut rx) = mpsc::channel::<String>(1000);

        let instance = Self {
            hashmap: hashmap_clone,
            remaining_remote_num: Arc::new(AtomicU64::new(0)),
            queue_remote_num: Arc::new(AtomicU64::new(0)),
            remaining_remote_bytes: Arc::new(AtomicU64::new(0)),
            queue_remote_bytes: Arc::new(AtomicU64::new(0)),
            namespace_id,
            table_name: table_name.clone(),
            init_successed: Arc::new(AtomicBool::new(false)),
            init_blob_successed: Arc::new(AtomicBool::new(false)),
            tx: Arc::new(tx),
            handle: None,
        };

        instance
    }
    /// Send document modification events to frontend
    ///
    /// #### Arguments
    /// * `live_event` - event
    /// * `tablename_hashmap` - collection storing table name and Table NameSpaceID
    /// * `binding_app_handle` - app_handle arc reference
    pub async fn emit_doc_edit<'a>(&mut self, live_event: LiveEvent) {
        let table_name = &self.table_name;
        let hashmap_store = &mut self.hashmap;
        match live_event {
            // remote modification
            LiveEvent::InsertRemote {
                content_status,
                from,
                entry,
            } => {
                // Only update if the we already have the content. Likely to happen when a remote user toggles "done".
                if content_status == ContentStatus::Complete {
                    println!("[doc_subscribe]Remote {} incoming file{:?}", from, entry);
                }
                println!(
                    "[doc_subscribe]{}:{:?}-{:?}",
                    table_name.clone(),
                    content_status,
                    entry
                );
                // resource table does not return progress
                if table_name.as_str() == "resource" {
                    return;
                }
                // get short hash
                let conetent_hash = entry.record().content_hash().fmt_short();
                let content_key = entry.key();
                let content_size = entry.record().content_len();
                let key = from_utf8(&content_key).unwrap().to_string();
                // if download data is 0, it means the data has been deleted and should not be added to download list
                if content_size == 0 {
                    if self.init_blob_successed.load(Ordering::Relaxed) {
                        // send delete data event to editor and main
                    }
                    println!(
                        "[doc_subscribe]Empty data {}:{:?}-{:?}",
                        table_name.clone(),
                        content_status,
                        entry
                    );
                    return;
                }

                // record state here for each remote update, then send message to frontend after data loading succeeds
                let mut hashmap = hashmap_store.lock().await;
                hashmap
                    .entry(conetent_hash.to_string())
                    .or_insert(RemoteUpdateData {
                        key,
                        size: content_size,
                        table_name: table_name.to_owned(),
                    });

                // record state when system is not initialized successfully
                if !self.init_successed.load(Ordering::SeqCst) {
                    // record total sync data from remote
                    self.remaining_remote_num.fetch_add(1, Ordering::SeqCst);
                    self.remaining_remote_bytes
                        .fetch_add(content_size, Ordering::SeqCst);

                    self.queue_remote_num.fetch_add(1, Ordering::SeqCst);
                    self.queue_remote_bytes
                        .fetch_add(content_size, Ordering::SeqCst);
                }
            }
            // local modification
            LiveEvent::InsertLocal { entry } => {
                println!(
                    "[doc_subscribe]{} Local file modification{:?}",
                    table_name.clone(),
                    entry
                );
            }
            LiveEvent::ContentReady { hash } => {
                println!(
                    "[doc_subscribe]{} starting download {}",
                    table_name.clone(),
                    hash
                );
                // get short hash
                let conetent_hash = hash.fmt_short();
                let mut hashmap = hashmap_store.lock().await;
                let rud = hashmap.get(&conetent_hash.to_string());
                if rud.is_none() {
                    println!(
                        "[doc_subscribe]{} file download successful {}, but not recorded in hashmap_store",
                        table_name.clone(),
                        hash
                    );
                    return;
                }
                // notify client of data changes after data download completes
                if let Some((_, remote_update_data)) =
                    hashmap.remove_entry(&conetent_hash.to_string())
                {
                    // record state when system is not initialized successfully
                    if !self.init_blob_successed.load(Ordering::SeqCst) {
                        let _ = self.tx.send(remote_update_data.clone().key).await;
                        self.queue_remote_num.fetch_sub(1, Ordering::SeqCst);
                        self.queue_remote_bytes
                            .fetch_sub(remote_update_data.size, Ordering::SeqCst);
                    }
                }
                println!(
                    "[doc_subscribe]{} file download successful {}",
                    table_name.clone(),
                    hash
                );
            }
            // this method executes when system loads for the first time
            LiveEvent::PendingContentReady => {
                // this method can be used as an indicator of whether loading is successful, including all files in blob
                let pre_init_blob_successed = self.init_blob_successed.swap(true, Ordering::SeqCst);
                println!(
                    "[doc_subscribe]{} all remote files synced successfully, {}",
                    table_name.clone(),
                    &pre_init_blob_successed
                );
                if !pre_init_blob_successed {}
                // end initialization method
                let handle = self.handle.as_ref();
                if handle.is_some() {
                    handle.unwrap().abort();
                    self.handle = None;
                }
            }
            LiveEvent::NeighborUp(public_key) => {
                println!("[doc_subscribe]New user {public_key}");
            }
            LiveEvent::NeighborDown(public_key) => {
                println!("[doc_subscribe]User exited {public_key}");
            }
            // this method executes when system loads for the first time
            LiveEvent::SyncFinished(sync_event) => {
                // this method can be used as an indicator of whether table loading is successful, not including files in blob
                // the method for successful blob download is [`LiveEvent::PendingContentReady`]
                self.init_successed.store(true, Ordering::SeqCst);
                println!(
                    "[doc_subscribe]{} transfer completed {:?}",
                    table_name.clone(),
                    sync_event
                );
            }
        }
    }
}
