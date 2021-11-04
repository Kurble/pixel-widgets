use std::cell::Cell;
use std::ops::Deref;
use std::sync::Arc;

use smallvec::SmallVec;

use crate::bitset::BitSet;
use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::node::GenericNode;
use crate::prelude::{StateVec, Style, Widget};
use crate::style::tree::Query;
use crate::style::Stylesheet;
use crate::tracker::ManagedStateTracker;
use crate::widget::Context;

/// Generic ui widget.
pub struct WidgetNode<'a, Message, W: Widget<'a, Message>> {
    widget: W,
    key: u64,
    widget_state: Option<&'a mut W::State>,
    size: Cell<Option<(Size, Size)>>,
    focused: Cell<Option<bool>>,
    position: (usize, usize),
    style: Option<Arc<Style>>,
    selector_matches: BitSet,
    stylesheet: Option<Arc<Stylesheet>>,
    class: Option<&'a str>,
    state: StateVec,
}

impl<'a, Message, W: Widget<'a, Message>> WidgetNode<'a, Message, W> {
    pub fn new(widget: W) -> Self {
        let key = widget.key();
        Self {
            widget,
            key,
            widget_state: None,
            size: Cell::new(None),
            focused: Cell::new(None),
            position: (0, 1),
            style: None,
            selector_matches: BitSet::new(),
            stylesheet: None,
            class: None,
            state: SmallVec::new(),
        }
    }
}

impl<'a, Message, W: Widget<'a, Message>> GenericNode<'a, Message> for WidgetNode<'a, Message, W> {
    fn get_key(&self) -> u64 {
        self.key
    }

    fn set_key(&mut self, key: u64) {
        self.key = key;
    }

    fn set_class(&mut self, class: &'a str) {
        self.class = Some(class);
    }

    fn acquire_state(&mut self, tracker: &mut ManagedStateTracker<'a>) {
        self.widget_state = Some(tracker.begin(self.key, || self.widget.mount()));
        self.widget.visit_children(&mut |child| {
            child.acquire_state(&mut *tracker);
        });
        tracker.end();
    }

    fn size(&self) -> (Size, Size) {
        if self.size.get().is_none() {
            let state = self.widget_state.as_ref().unwrap();
            let style = self.stylesheet.as_ref().unwrap().deref();
            let mut size = self.widget.size(&**state, style);
            size.0 = match size.0 {
                Size::Exact(size) => Size::Exact(size + style.margin.left + style.margin.right),
                other => other,
            };
            size.1 = match size.1 {
                Size::Exact(size) => Size::Exact(size + style.margin.top + style.margin.bottom),
                other => other,
            };
            self.size.replace(Some(size));
        }
        self.size.get().unwrap()
    }

    fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32) -> bool {
        let state = self.widget_state.as_ref().unwrap();
        let stylesheet = self.stylesheet.as_ref().unwrap().deref();
        let layout = layout.after_padding(stylesheet.margin);
        self.widget.hit(&**state, layout, clip, stylesheet, x, y)
    }

    fn focused(&self) -> bool {
        if self.focused.get().is_none() {
            let state = self.widget_state.as_ref().unwrap();
            self.focused.replace(Some(self.widget.focused(&**state)));
        }
        self.focused.get().unwrap()
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        let state = self.widget_state.as_mut().unwrap();
        let stylesheet = self.stylesheet.as_ref().unwrap().deref();
        let layout = layout.after_padding(stylesheet.margin);

        self.widget.draw(&mut **state, layout, clip, stylesheet)
    }

    fn style(&mut self, query: &mut Query, position: (usize, usize)) {
        self.position = position;

        // remember style
        self.style = Some(query.style.clone());

        // resolve own stylesheet
        self.state = self.widget.state(&**self.widget_state.as_ref().unwrap());
        self.selector_matches = query.match_widget(
            self.widget.widget(),
            self.class.unwrap_or(""),
            self.state.as_slice(),
            self.position.0,
            self.position.1,
        );
        self.stylesheet.replace(query.style.get(&self.selector_matches));

        // resolve children style
        query.ancestors.push(self.selector_matches.clone());
        let own_siblings = std::mem::take(&mut query.siblings);
        let mut i = 0;
        let len = self.widget.len();
        self.widget.visit_children(&mut |child| {
            child.style(&mut *query, (i, len));
            i += 1;
        });
        query.siblings = own_siblings;
        query.siblings.push(query.ancestors.pop().unwrap());
    }

    fn add_matches(&mut self, query: &mut Query) {
        let additions = query.match_widget(
            self.widget.widget(),
            self.class.unwrap_or(""),
            self.state.as_slice(),
            self.position.0,
            self.position.1,
        );

        let new_style = self.selector_matches.union(&additions);
        if new_style != self.selector_matches {
            self.selector_matches = new_style;
            self.stylesheet
                .replace(self.style.as_ref().unwrap().get(&self.selector_matches));
        }

        query.ancestors.push(additions);
        let own_siblings = std::mem::take(&mut query.siblings);
        self.widget.visit_children(&mut |child| child.add_matches(&mut *query));
        query.siblings = own_siblings;
        query.siblings.push(query.ancestors.pop().unwrap());
    }

    fn remove_matches(&mut self, query: &mut Query) {
        let removals = query.match_widget(
            self.widget.widget(),
            self.class.unwrap_or(""),
            self.state.as_slice(),
            self.position.0,
            self.position.1,
        );

        let new_style = self.selector_matches.difference(&removals);
        if new_style != self.selector_matches {
            self.selector_matches = new_style;
            self.stylesheet
                .replace(self.style.as_ref().unwrap().get(&self.selector_matches));
        }

        query.ancestors.push(removals);
        let own_siblings = std::mem::take(&mut query.siblings);
        self.widget
            .visit_children(&mut |child| child.remove_matches(&mut *query));
        query.siblings = own_siblings;
        query.siblings.push(query.ancestors.pop().unwrap());
    }

    fn event(&mut self, layout: Rectangle, clip: Rectangle, event: Event, context: &mut Context<Message>) {
        let state = self.widget_state.as_mut().unwrap();
        let stylesheet = self.stylesheet.as_ref().unwrap().deref();
        let layout = layout.after_padding(stylesheet.margin);

        self.widget
            .event(&mut **state, layout, clip, stylesheet, event, context);

        let next_state = self.widget.state(&**state);
        if next_state != self.state {
            self.state = next_state;

            // find out if the style changed as a result of the state change
            let new_style = self.style.as_ref().unwrap().rule_tree().rematch(
                &self.selector_matches,
                self.state.as_slice(),
                self.class.unwrap_or(""),
                self.position.0,
                self.position.1,
            );

            // apply the style change to self and any children that have styles living down the same rule tree paths.
            if new_style != self.selector_matches {
                context.redraw();

                let difference = new_style.difference(&self.selector_matches);
                let additions = difference.intersection(&new_style);
                let removals = difference.intersection(&self.selector_matches);

                if !additions.is_empty() {
                    let mut query = Query {
                        style: self.style.clone().unwrap(),
                        ancestors: vec![additions],
                        siblings: vec![],
                    };
                    self.widget.visit_children(&mut |child| child.add_matches(&mut query));
                }

                if !removals.is_empty() {
                    let mut query = Query {
                        style: self.style.clone().unwrap(),
                        ancestors: vec![removals],
                        siblings: vec![],
                    };
                    self.widget
                        .visit_children(&mut |child| child.remove_matches(&mut query));
                }

                self.selector_matches = new_style;
                self.stylesheet
                    .replace(self.style.as_ref().unwrap().get(&self.selector_matches));
            }
        }

        self.focused
            .replace(Some(self.widget.focused(&**self.widget_state.as_ref().unwrap())));
    }

    fn poll(&mut self, context: &mut Context<Message>) {
        self.widget.visit_children(&mut |child| child.poll(context));
    }
}
