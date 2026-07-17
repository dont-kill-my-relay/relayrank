use serde::{Deserialize, Serialize};

use crate::tor_status::RelayId;

pub mod network_metric;
pub mod relay_metric;
pub mod tor_status;

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
struct Mapping {
    client_guard: Vec<RelayId>,
    exit_destination: Vec<RelayId>,
}
