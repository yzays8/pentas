use crate::renderer::RenderObject;

#[derive(Debug, Clone)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, query: &str, objects: &[RenderObject]) {
        let query = query.to_owned();
        let objects = objects.to_owned();
        self.entries.push(HistoryEntry { query, objects });
    }

    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.entries.get(index)
    }
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub query: String,
    pub objects: Vec<RenderObject>,
}
