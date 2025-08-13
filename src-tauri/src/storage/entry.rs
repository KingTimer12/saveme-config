use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    pub target_hint: String,  // ex: "app:zed:settings"
    pub logical_path: String, // ex: "config/zed/settings.json"
    pub blob_id: String,
    pub tar_member: Option<String>,
}
