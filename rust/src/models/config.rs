use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallArgs {
    pub name:         String,
    pub display:      String,
    pub description:  String,
    pub script:       String,
    pub node_args:    String,
    pub env:          String,
    pub working_dir:  String,
    pub log_file:     String,
    pub start_type:   String,
    pub auto_restart: bool,
}
