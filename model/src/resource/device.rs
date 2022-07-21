use nmos_rs_schema::is_04;
use uuid::Uuid;

use crate::tai::TaiTime;

use super::{Node, Resource};

#[derive(Debug)]
pub struct Device {
    pub id: Uuid,
    pub version: TaiTime,
    pub label: String,
    pub type_: String,
    pub node_id: Uuid,
    pub senders: Vec<Uuid>,
    pub receivers: Vec<Uuid>,
}

impl Device {
    pub fn builder<S: Into<String>>(node: &Node, device_type: S) -> DeviceBuilder {
        DeviceBuilder::new(node, device_type)
    }
}

pub struct DeviceBuilder {
    label: Option<String>,
    type_: String,
    node_id: Uuid,
}

impl DeviceBuilder {
    pub fn new<S: Into<String>>(node: &Node, device_type: S) -> DeviceBuilder {
        DeviceBuilder {
            label: None,
            type_: device_type.into(),
            node_id: node.id,
        }
    }

    pub fn label(mut self, label: String) -> DeviceBuilder {
        self.label = Some(label);
        self
    }

    pub fn build(self) -> Device {
        Device {
            id: Uuid::new_v4(),
            version: TaiTime::now(),
            label: self.label.unwrap_or(String::new()),
            type_: self.type_,
            node_id: self.node_id,
            senders: Vec::new(),
            receivers: Vec::new(),
        }
    }
}

impl Resource for Device {
    type JsonType = is_04::v1_0_x::DeviceJson;

    fn to_json(&self) -> Self::JsonType {
        // Senders
        let senders = self.senders.iter().map(|s| s.to_string()).collect();

        // Receivers
        let receivers = self.receivers.iter().map(|r| r.to_string()).collect();

        is_04::v1_0_x::DeviceJson {
            id: self.id.to_string(),
            version: self.version.to_string(),
            label: self.label.clone(),
            type_: self.type_.clone(),
            node_id: self.node_id.to_string(),
            senders,
            receivers,
        }
    }
}