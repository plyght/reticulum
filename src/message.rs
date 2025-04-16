use crate::constants::FIELD_SPLITTER;

#[derive(Clone, Debug)]
pub struct Message {
    content: String,
    sender_name: String,
    sender_ip: String,
}

impl Message {
    pub fn new(content: String, sender_name: String, sender_ip: String) -> Self {
        Self {
            content,
            sender_name,
            sender_ip,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn sender_name(&self) -> &str {
        &self.sender_name
    }

    pub fn sender_ip(&self) -> &str {
        &self.sender_ip
    }

    pub fn encode_for_broadcast(&self) -> String {
        format!(
            "{}{}{}{}{}",
            self.sender_name, FIELD_SPLITTER, self.sender_ip, FIELD_SPLITTER, self.content
        )
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.encode_for_broadcast())
    }
}
