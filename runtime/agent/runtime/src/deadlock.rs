use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct DeadlockDetector {
    graph: HashMap<String, Vec<String>>,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            graph: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.graph
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    pub fn detect_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.graph.keys() {
            if self.dfs(node, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    fn dfs(
        &self,
        node: &String,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        if let Some(neighbors) = self.graph.get(node) {
            for neighbor in neighbors {
                if self.dfs(neighbor, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}
