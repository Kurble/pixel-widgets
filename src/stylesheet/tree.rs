use crate::stylesheet::{Selector, Rule, SelectorWidget, Style};
use std::rc::Rc;
use crate::bitset::BitSet;
use std::iter::FromIterator;

pub(crate) struct RuleTree {
    nodes: Vec<RuleNode>,
}

pub(crate) struct RuleNode {
    selector: Selector,
    rules: Vec<Rule>,
    children: Vec<usize>,
}

pub(crate) struct NewRuleTree {
    selector: Selector,
    rules: Vec<Rule>,
    children: Vec<NewRuleTree>,
}

#[derive(Clone)]
pub(crate) struct Query {
    pub style: Rc<Style>,
    pub ancestors: Vec<BitSet>,
    pub siblings: Vec<BitSet>,
}

impl RuleTree {
    pub fn iter_rules<'a>(&'a self, style: &'a BitSet) -> impl Iterator<Item = &'a Rule> {
        style.iter().flat_map(move |node| self.nodes[node].rules.iter())
    }

    /// Add a node from the rule tree to a bitset.
    /// This will also add all the `:` based child selectors that apply based on `state`, `n` and `last`.
    pub fn add_to_bitset(&self, node: usize, state: &str, class: &str, n: usize, len: usize, to: &mut BitSet) {
        to.insert(node);
        for &child in self.nodes[node].children.iter() {
            let add = match self.nodes[child].selector {
                Selector::State(ref sel_state) => sel_state == state,
                Selector::Class(ref sel_class) => sel_class == class,
                Selector::Nth(num) => n == num,
                Selector::NthMod(num, den) => (n % den) == num,
                Selector::NthLast(num) => len - 1 - n == num,
                Selector::NthLastMod(num, den) => ((len - 1 - n) % den) == num,
                _ => false,
            };
            if add {
                self.add_to_bitset(child, state, class, n, len, to);
            }
        }
    }

    /// Match a child widget of a widget matched to this rule tree.
    pub fn match_child<'a>(
        &'a self,
        node: usize,
        direct: bool,
        widget: &'a str,
    ) -> impl 'a + Iterator<Item = usize> {
        self.nodes[node]
            .children
            .iter()
            .filter_map(move |&tree| match self.nodes[tree].selector {
                Selector::Widget(ref sel_widget) => Some(tree).filter(|_| sel_widget.matches(widget)),
                Selector::WidgetDirectChild(ref sel_widget) => {
                    Some(tree).filter(|_| direct && sel_widget.matches(widget))
                }
                _ => None,
            })
    }

    /// Match a sibling widget of a widget matched to this rule tree.
    pub fn match_sibling<'a>(&'a self, node: usize, direct: bool, widget: &'a str) -> impl 'a + Iterator<Item = usize> {
        self.nodes[node]
            .children
            .iter()
            .filter_map(move |&tree| match self.nodes[tree].selector {
                Selector::WidgetDirectAfter(ref sel_widget) => {
                    Some(tree).filter(|_| direct && sel_widget.matches(widget))
                }
                Selector::WidgetAfter(ref sel_widget) => Some(tree).filter(|_| sel_widget.matches(widget)),
                _ => None,
            })
    }

    pub fn restyle(&self, style: &BitSet, state: &str, class: &str, n: usize, len: usize) -> BitSet {
        let mut result = BitSet::new();

        for selector in style.iter() {
            let keep = match self.nodes[selector].selector {
                Selector::State(ref sel_state) => sel_state == state,
                _ => true,
            };
            if keep {
                self.add_to_bitset(selector, state, class, n, len, &mut result);
            }
        }

        result
    }
}

impl Default for RuleTree {
    fn default() -> Self {
        Self {
            nodes: vec![RuleNode {
                selector: Selector::Widget(SelectorWidget::Any),
                rules: vec![],
                children: vec![],
            }],
        }
    }
}

impl NewRuleTree {
    pub fn new(selector: Selector) -> Self {
        NewRuleTree {
            selector,
            rules: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Recursively insert some rules at the selectors path
    pub fn insert(&mut self, selectors: impl AsRef<[Selector]>, rules: Vec<Rule>) {
        match selectors.as_ref().get(0) {
            None => self.rules.extend(rules),
            Some(selector) => {
                let mut index = self.children.len();

                for (i, node) in self.children.iter().enumerate() {
                    if &node.selector == selector {
                        index = i;
                        break;
                    }
                }

                if index == self.children.len() {
                    self.children.push(NewRuleTree {
                        selector: selector.clone(),
                        rules: Vec::new(),
                        children: Vec::new(),
                    });
                }

                self.children[index].insert(&selectors.as_ref()[1..], rules);
            }
        }
    }

    fn flatten(self, into: &mut RuleTree) -> usize {
        let index = into.nodes.len();
        into.nodes.push(RuleNode {
            selector: self.selector,
            rules: self.rules,
            children: Vec::new(),
        });

        for child in self.children {
            let child = child.flatten(into);
            into.nodes[index].children.push(child);
        }
        index
    }
}

impl Into<RuleTree> for NewRuleTree {
    fn into(self) -> RuleTree {
        let mut result = RuleTree { nodes: Vec::new() };
        self.flatten(&mut result);
        result
    }
}

impl Query {
    pub fn from_style(style: Rc<Style>) -> Self {
        Self {
            style,
            ancestors: vec![BitSet::from_iter(Some(0))],
            siblings: Vec::new(),
        }
    }

    pub fn match_widget(&self, widget: &str, class: &str, state: &str, n: usize, len: usize) -> BitSet {
        let mut result = BitSet::new();

        let from_ancestors = self.ancestors.iter().rev().enumerate().flat_map(move |(i, matches)| {
            matches
                .iter()
                .flat_map(move |node| self.style.rule_tree.match_child(node, i == 0, widget))
        });
        let from_siblings = self.siblings.iter().rev().enumerate().flat_map(move |(i, matches)| {
            matches
                .iter()
                .flat_map(move |node| self.style.rule_tree.match_sibling(node, i == 0, widget))
        });

        for node in from_ancestors.chain(from_siblings) {
            self.style.rule_tree.add_to_bitset(node, state, class, n, len, &mut result);
        }

        result
    }
}