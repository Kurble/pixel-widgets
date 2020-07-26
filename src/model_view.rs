use crate::widget::{Node};
use crate::stylesheet::Style;
use crate::stylesheet::tree::Query;
use crate::Model;
use std::rc::Rc;

pub struct ModelView<M: Model> {
    model: Box<M>,
    view: Option<Node<'static, M::Message>>,
}

impl<M: Model> ModelView<M> {
    pub fn new(model: M) -> Self {
        Self {
            model: Box::new(model),
            view: None,
        }
    }

    pub fn set_dirty(&mut self) {
        self.view = None;
    }

    pub fn dirty(&self) -> bool {
        self.view.is_none()
    }

    pub fn model(&self) -> &M {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut M {
        self.view = None;
        &mut self.model
    }

    pub fn view(&mut self, style: Rc<Style>) -> &mut Node<'static, M::Message> {
        if self.view.is_none() {
            unsafe {
                let mut root = (self.model.as_mut() as *mut M).as_mut().unwrap().view();
                root.style(&mut Query::from_style(style));
                self.view.replace(root);
            }
        }
        self.view.as_mut().unwrap()
    }
}

impl<M: Model> Drop for ModelView<M> {
    fn drop(&mut self) {
        self.view = None;
    }
}
