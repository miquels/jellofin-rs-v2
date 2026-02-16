use crate::collection::CollectionRepo;
use crate::database::Repository;
use std::sync::Arc;

#[derive(Clone)]
pub struct JellyfinState {
    pub repo: Arc<dyn Repository>,
    pub collections: Arc<CollectionRepo>,
    pub server_id: String,
    pub server_name: String,
    pub image_resizer: Arc<crate::imageresize::ImageResizer>,
    pub config: Arc<crate::server::Config>,
}
