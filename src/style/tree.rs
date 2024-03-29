use crate::bitset::BitSet;
use crate::draw::Patch;
use crate::style::{Declaration, FontId, ImageId, PatchId, Selector, Style, StyleState};
use crate::text::Font;
use crate::widget::image::ImageData;
use std::collections::{HashMap, VecDeque};
use std::iter::FromIterator;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(crate) struct RuleTree {
    rules: Vec<Rule>,
}

#[derive(Debug)]
pub(crate) struct Rule {
    selector: Selector,
    declarations: Vec<Declaration<ImageData, Patch, Font>>,
    children: Vec<usize>,
}

#[derive(Debug)]
pub(crate) struct RuleTreeBuilder {
    pub selector: Selector,
    pub declarations: Vec<Declaration<ImageId, PatchId, FontId>>,
    pub children: Vec<RuleTreeBuilder>,
}

#[derive(Clone)]
pub struct Query {
    pub style: Arc<Style>,
    pub ancestors: Vec<BitSet>,
    pub siblings: Vec<BitSet>,
}

impl RuleTree {
    pub fn iter_declarations<'a>(
        &'a self,
        style: &'a BitSet,
    ) -> impl Iterator<Item = &'a Declaration<ImageData, Patch, Font>> {
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

impl RuleTreeBuilder {
    pub fn new() -> Self {
        RuleTreeBuilder {
            selector: Selector::Root,
            declarations: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Recursively insert some rules at the selectors path
    pub fn insert(&mut self, selectors: impl AsRef<[Selector]>, rules: Vec<Declaration<ImageId, PatchId, FontId>>) {
        self.select(selectors).declarations.extend(rules);
    }

    /// Get or create a `RuleTreeBuilder` at the selector path
    pub fn select(&mut self, selectors: impl AsRef<[Selector]>) -> &mut Self {
        match selectors.as_ref().get(0) {
            None => self,
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

                self.children[index].select(&selectors.as_ref()[1..])
            }
        }
    }

    /// Merge a `RuleTreeBuilder` into this one.
    /// It must have the same root selector.
    pub fn merge(&mut self, mut rule_tree: Self) {
        assert_eq!(self.selector, rule_tree.selector);

        // merge the declarations
        self.declarations.append(&mut rule_tree.declarations);

        // merge the children
        for sub_tree in rule_tree.children {
            if let Some(child) = self.children.iter_mut().find(|c| c.selector == sub_tree.selector) {
                child.merge(sub_tree);
            } else {
                self.children.push(sub_tree);
            }
        }
    }

    pub(crate) fn build(
        self,
        images: &HashMap<String, ImageData>,
        patches: &HashMap<String, Patch>,
        fonts: &HashMap<String, Font>,
    ) -> RuleTree {
        let mut rules = Vec::<Rule>::new();

        let mut queue = VecDeque::new();
        queue.push_back((self, None));

        while let Some((rule, parent)) = queue.pop_front() {
            for child in rule.children {
                queue.push_back((child, Some(rules.len())));
            }

            if let Some(parent) = parent {
                let child = rules.len();
                rules[parent].children.push(child);
            }

            rules.push(Rule {
                selector: rule.selector,
                declarations: rule
                    .declarations
                    .into_iter()
                    .map(|declaration| match declaration {
                        Declaration::BackgroundNone => Declaration::BackgroundNone,
                        Declaration::BackgroundColor(x) => Declaration::BackgroundColor(x),
                        Declaration::BackgroundImage(ImageId(x), y) => {
                            Declaration::BackgroundImage(images[&x].clone(), y)
                        }
                        Declaration::BackgroundPatch(PatchId(x), y) => {
                            Declaration::BackgroundPatch(patches[&x].clone(), y)
                        }
                        Declaration::Font(FontId(x)) => Declaration::Font(fonts[&x].clone()),
                        Declaration::Color(x) => Declaration::Color(x),
                        Declaration::Padding(x) => Declaration::Padding(x),
                        Declaration::PaddingLeft(x) => Declaration::PaddingLeft(x),
                        Declaration::PaddingRight(x) => Declaration::PaddingRight(x),
                        Declaration::PaddingTop(x) => Declaration::PaddingTop(x),
                        Declaration::PaddingBottom(x) => Declaration::PaddingBottom(x),
                        Declaration::Margin(x) => Declaration::Margin(x),
                        Declaration::MarginLeft(x) => Declaration::MarginLeft(x),
                        Declaration::MarginRight(x) => Declaration::MarginRight(x),
                        Declaration::MarginTop(x) => Declaration::MarginTop(x),
                        Declaration::MarginBottom(x) => Declaration::MarginBottom(x),
                        Declaration::TextSize(x) => Declaration::TextSize(x),
                        Declaration::TextBorder(x) => Declaration::TextBorder(x),
                        Declaration::TextWrap(x) => Declaration::TextWrap(x),
                        Declaration::Width(x) => Declaration::Width(x),
                        Declaration::Height(x) => Declaration::Height(x),
                        Declaration::LayoutDirection(x) => Declaration::LayoutDirection(x),
                        Declaration::AlignHorizontal(x) => Declaration::AlignHorizontal(x),
                        Declaration::AlignVertical(x) => Declaration::AlignVertical(x),
                        Declaration::AddFlag(x) => Declaration::AddFlag(x),
                        Declaration::RemoveFlag(x) => Declaration::RemoveFlag(x),
                    })
                    .collect(),
                children: Vec::new(),
            });
        }

        RuleTree { rules }
    }
}

impl Default for RuleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Query {
    pub fn from_style(style: Arc<Style>) -> Self {
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
