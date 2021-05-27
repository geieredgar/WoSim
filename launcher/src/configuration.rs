use serde::{Deserialize, Serialize};
use uuid::Uuid;
use whoami::username;

#[derive(Serialize, Deserialize, Default)]
pub struct Configuration {
    pub local: Local,
}

#[derive(Serialize, Deserialize)]
pub struct Local {
    pub uuid: Uuid,
    pub username: String,
}

impl Default for Local {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            username: username(),
        }
    }
}
