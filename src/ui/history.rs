use crate::ui::object::RenderObject;

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

    pub fn add(&mut self, query: String, objects: Vec<RenderObject>) {
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
