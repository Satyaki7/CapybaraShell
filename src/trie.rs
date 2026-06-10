use std::collections::HashMap;

pub struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_end: false,
        }
    }
}

pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }
    // inserting the builtin commands
    pub fn insert(&mut self, word: &str) {
        let mut current = &mut self.root;

        for ch in word.chars() {
            current = current
                .children
                .entry(ch)
                .or_insert_with(TrieNode::new);
        }

        current.is_end = true;
    }

        fn find_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut current = &self.root;

        for ch in prefix.chars() {
            match current.children.get(&ch) {
                Some(node) => current = node,
                None => return None,
            }
        }

        Some(current)
    }


    pub fn autocomplete(&self, prefix: &str) -> Option<String> {
        let mut result = prefix.to_string();
        let mut current = self.find_node(prefix)?;

        loop {
            if current.is_end || current.children.len() != 1 {
                break;
            }

            let (ch, next) = current.children.iter().next().unwrap();
            result.push(*ch);
            current = next;
        }

        if result == prefix && !current.is_end && current.children.is_empty() {
             return None;
        }

        Some(result)
    }

    pub fn get_matches(&self, prefix: &str) -> Vec<String> {
        let node = match self.find_node(prefix) {
            Some(node) => node,
            None => return Vec::new(),
        };

        let mut matches = Vec::new();
        self.collect_matches(node, prefix, &mut matches);
        matches.sort();
        matches.dedup();
        matches
    }

    fn collect_matches(&self, node: &TrieNode, current: &str, matches: &mut Vec<String>) {
        if node.is_end {
            matches.push(current.to_string());
        }

        for (ch, next) in &node.children {
            let mut next_current = current.to_string();
            next_current.push(*ch);
            self.collect_matches(next, &next_current, matches);
        }
    }
}
