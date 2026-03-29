use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Social {
    pub platform: String,
    pub url: String,
    pub username: Option<String>,
}

impl PartialEq for Social {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for Social {}

impl std::hash::Hash for Social {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub url: String,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub socials: Vec<Social>,
    pub names: Vec<String>,
}

impl Lead {
    pub fn new(url: String) -> Self {
        Self {
            url,
            emails: Vec::new(),
            phones: Vec::new(),
            socials: Vec::new(),
            names: Vec::new(),
        }
    }

    /// Returns true if this lead has any extracted data.
    pub fn has_data(&self) -> bool {
        !self.emails.is_empty()
            || !self.phones.is_empty()
            || !self.socials.is_empty()
            || !self.names.is_empty()
    }

    /// Deduplicate all fields in place.
    pub fn dedup(&mut self) {
        dedup_vec(&mut self.emails);
        dedup_vec(&mut self.phones);
        dedup_vec(&mut self.names);

        let mut seen = HashSet::new();
        self.socials.retain(|s| seen.insert(s.url.clone()));
    }
}

fn dedup_vec(v: &mut Vec<String>) {
    let mut seen = HashSet::new();
    v.retain(|item| seen.insert(item.to_lowercase()));
}
