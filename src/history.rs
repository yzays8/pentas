use anyhow::{bail, Result};

use crate::renderer::RenderObject;

#[derive(Debug, Clone)]
pub struct History {
    back_stack: Vec<HistoryEntry>,
    forward_stack: Vec<HistoryEntry>,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    pub fn new() -> Self {
        Self {
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
        }
    }

    pub fn with_initial_page(query: &str, objects: &[RenderObject]) -> Self {
        let mut history = Self::new();
        history.add(query, objects);
        history
    }

    pub fn add(&mut self, query: &str, objects: &[RenderObject]) {
        let query = query.to_owned();
        let objects = objects.to_owned();
        if !self.forward_stack.is_empty() {
            self.forward_stack.clear();
        }
        self.back_stack.push(HistoryEntry { query, objects });
    }

    pub fn forward(&mut self) -> Result<HistoryEntry> {
        if !self.is_forwardable() {
            bail!("No history");
        }
        let entry = self.forward_stack.pop().unwrap();
        self.back_stack.push(entry.clone());
        Ok(entry)
    }

    pub fn rewind(&mut self) -> Result<HistoryEntry> {
        if !self.is_rewindable() {
            bail!("No history");
        }
        self.forward_stack.push(self.back_stack.pop().unwrap());
        Ok(self.back_stack.last().unwrap().clone())
    }

    pub fn is_rewindable(&self) -> bool {
        self.back_stack.len() > 1
    }

    pub fn is_forwardable(&self) -> bool {
        !self.forward_stack.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub query: String,
    pub objects: Vec<RenderObject>,
}
