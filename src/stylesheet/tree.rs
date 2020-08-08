use crate::bitset::BitSet;
use crate::stylesheet::{Declaration, Selector, SelectorWidget, StyleState, Style};
use std::iter::FromIterator;
use std::rc::Rc;

pub(crate) struct RuleTree {
    rules: Vec<Rule>,
}

pub(crate) struct Rule {
    selector: Selector,
    declarations: Vec<Declaration>,
    children: Vec<usize>,
}

pub(crate) struct RuleTreeBuilder {
    selector: Selector,
    declarations: Vec<Declaration>,
    children: Vec<RuleTreeBuilder>,
}

#[derive(Clone)]
pub(crate) struct Query {
    pub style: Rc<Style>,
    pub ancestors: Vec<BitSet>,
    pub siblings: Vec<BitSet>,
}

impl RuleTree {
    pub fn iter_declarations<'a>(&'a self, style: &'a BitSet) -> impl Iterator<Item = &'a Declaration> {
        style.iter().flat_map(move |rule| self.rules[rule].declarations.iter())
    }

    /// Add a node from the rule tree to a bitset.
    /// This will also add all the `:` based child selectors that apply based on `state`, `n` and `last`.
    pub fn add_to_bitset<S: AsRef<str>>(
        &self,
        rule: usize,
        state: &[StyleState<S>],
        class: &str,
        n: usize,
        len: usize,
        to: &mut BitSet,
    ) {
        to.insert(rule);
        for &child_rule in self.rules[rule].children.iter() {
            if self.rules[child_rule]
                .selector
                .match_meta(state, class, n, len)
                .unwrap_or(false)
            {
                self.add_to_bitset(child_rule, state, class, n, len, to);
            }
        }
    }

    /// Match a child widget of a widget matched to this rule tree.
    pub fn match_child<'a>(&'a self, rule: usize, direct: bool, widget: &'a str) -> impl 'a + Iterator<Item = usize> {
        self.rules[rule].children.iter().cloned().filter(move |&child_rule| {
            self.rules[child_rule]
                .selector
                .match_child(direct, widget)
                .unwrap_or(false)
        })
    }

    /// Match a sibling widget of a widget matched to this rule tree.
    pub fn match_sibling<'a>(&'a self, rule: usize, direct: bool, widget: &'a str) -> impl 'a + Iterator<Item = usize> {
        self.rules[rule].children.iter().cloned().filter(move |&child_rule| {
            self.rules[child_rule]
                .selector
                .match_sibling(direct, widget)
                .unwrap_or(false)
        })
    }

    /// Perform meta-selector matching again for the rules in the given bitset.
    /// Non meta selectors will be retained no matter what.
    pub fn rematch<S: AsRef<str>>(
        &self,
        style: &BitSet,
        state: &[StyleState<S>],
        class: &str,
        n: usize,
        len: usize,
    ) -> BitSet {
        let mut result = BitSet::new();

        for selector in style.iter().filter(|&selector| {
            self.rules[selector]
                .selector
                .match_meta(state, class, n, len)
                .unwrap_or(/*non meta selectors need to be retained: */ true)
        }) {
            self.add_to_bitset(selector, state, class, n, len, &mut result);
        }

        result
    }
}

impl Default for RuleTree {
    fn default() -> Self {
        Self {
            rules: vec![Rule {
                selector: Selector::Widget(SelectorWidget::Any),
                declarations: vec![],
                children: vec![],
            }],
        }
    }
}

impl RuleTreeBuilder {
    pub fn new(selector: Selector) -> Self {
        RuleTreeBuilder {
            selector,
            declarations: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Recursively insert some rules at the selectors path
    pub fn insert(&mut self, selectors: impl AsRef<[Selector]>, rules: Vec<Declaration>) {
        match selectors.as_ref().get(0) {
            None => self.declarations.extend(rules),
            Some(selector) => {
                let mut index = self.children.len();

                for (i, node) in self.children.iter().enumerate() {
                    if &node.selector == selector {
                        index = i;
                        break;
                    }
                }

                if index == self.children.len() {
                    self.children.push(RuleTreeBuilder {
                        selector: selector.clone(),
                        declarations: Vec::new(),
                        children: Vec::new(),
                    });
                }

                self.children[index].insert(&selectors.as_ref()[1..], rules);
            }
        }
    }

    fn flatten(self, into: &mut RuleTree) -> usize {
        let index = into.rules.len();
        into.rules.push(Rule {
            selector: self.selector,
            declarations: self.declarations,
            children: Vec::new(),
        });

        for child in self.children {
            let child = child.flatten(into);
            into.rules[index].children.push(child);
        }
        index
    }
}

impl Into<RuleTree> for RuleTreeBuilder {
    fn into(self) -> RuleTree {
        let mut result = RuleTree { rules: Vec::new() };
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

    pub fn match_widget<S: AsRef<str>>(
        &self,
        widget: &str,
        class: &str,
        state: &[StyleState<S>],
        n: usize,
        len: usize,
    ) -> BitSet {
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
            self.style
                .rule_tree
                .add_to_bitset(node, state, class, n, len, &mut result);
        }

        result
    }
}
