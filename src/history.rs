use crate::renderer::RenderObjectsInfo;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub query: String,
    pub objs_info: RenderObjectsInfo,
}

#[derive(Debug, Clone, Default)]
pub struct History {
    current: Option<HistoryEntry>,
    back_stack: Vec<HistoryEntry>,
    forward_stack: Vec<HistoryEntry>,
    unreachable_stack: Vec<HistoryEntry>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_initial_page(query: &str, objs_info: &RenderObjectsInfo) -> Self {
        let mut history = Self::new();
        history.add(query, objs_info);
        history
    }

    pub fn get_current(&self) -> Option<&HistoryEntry> {
        self.current.as_ref()
    }

    pub fn add(&mut self, query: &str, objs_info: &RenderObjectsInfo) {
        while let Some(e) = self.forward_stack.pop() {
            self.unreachable_stack.push(e);
        }
        if let Some(current) = self.current.take() {
            self.back_stack.push(current);
        }
        self.current = Some(HistoryEntry {
            query: query.to_owned(),
            objs_info: objs_info.clone(),
        });
    }

    pub fn forward(&mut self) -> Option<&HistoryEntry> {
        if !self.is_forwardable() {
            return None;
        }
        if let Some(current) = self.current.take() {
            self.back_stack.push(current);
        }
        self.current = self.forward_stack.pop();
        self.current.as_ref()
    }

    pub fn rewind(&mut self) -> Option<&HistoryEntry> {
        if !self.is_rewindable() {
            return None;
        }
        if let Some(current) = self.current.take() {
            self.forward_stack.push(current);
        }
        self.current = self.back_stack.pop();
        self.current.as_ref()
    }

    pub fn is_rewindable(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn is_forwardable(&self) -> bool {
        !self.forward_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_page() {
        let history = History::with_initial_page("p1", &RenderObjectsInfo::default());
        assert_eq!(history.get_current().unwrap().query, "p1");
        assert!(!history.is_rewindable());
        assert!(!history.is_forwardable());
    }

    #[test]
    fn add_page() {
        let mut history = History::with_initial_page("p1", &RenderObjectsInfo::default());
        history.add("p2", &RenderObjectsInfo::default());
        assert_eq!(history.get_current().unwrap().query, "p2");
        assert!(history.is_rewindable());
        assert!(!history.is_forwardable());
    }

    #[test]
    fn back_and_forth() {
        let mut history = History::with_initial_page("p1", &RenderObjectsInfo::default());
        history.add("p2", &RenderObjectsInfo::default());
        history.add("p3", &RenderObjectsInfo::default());

        // back
        assert_eq!(history.rewind().unwrap().query, "p2");
        assert_eq!(history.get_current().unwrap().query, "p2");
        assert!(history.is_rewindable());
        assert!(history.is_forwardable());

        // back
        assert_eq!(history.rewind().unwrap().query, "p1");
        assert_eq!(history.get_current().unwrap().query, "p1");
        assert!(!history.is_rewindable());
        assert!(history.is_forwardable());

        // forward
        assert_eq!(history.forward().unwrap().query, "p2");
        assert_eq!(history.get_current().unwrap().query, "p2");
        assert!(history.is_rewindable());
        assert!(history.is_forwardable());

        // forward
        assert_eq!(history.forward().unwrap().query, "p3");
        assert_eq!(history.get_current().unwrap().query, "p3");
        assert!(!history.is_forwardable());
        assert!(history.is_rewindable());
    }

    #[test]
    fn add_page_after_rewind() {
        let mut history = History::with_initial_page("p1", &RenderObjectsInfo::default());
        history.add("p2", &RenderObjectsInfo::default());
        history.rewind();

        // add (p2 is unreachable)
        history.add("p3", &RenderObjectsInfo::default());
        assert_eq!(history.get_current().unwrap().query, "p3");
        assert!(!history.is_forwardable());
        assert!(history.is_rewindable());

        // back
        assert_eq!(history.rewind().unwrap().query, "p1");
        assert_eq!(history.get_current().unwrap().query, "p1");
        assert!(history.is_forwardable());
        assert!(!history.is_rewindable());

        // forward
        assert_eq!(history.forward().unwrap().query, "p3");
        assert_eq!(history.get_current().unwrap().query, "p3");
        assert!(!history.is_forwardable());
        assert!(history.is_rewindable());
    }
}
