use crate::stylesheet::tree::Query;
use crate::stylesheet::Style;
use crate::widget::Node;
use crate::Model;
use std::sync::{Arc, Mutex};

pub struct ModelView<M: Model> {
    model: Box<M>,
    view: Option<Mutex<Node<'static, M::Message>>>,
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

    pub fn view(&mut self, style: Arc<Style>) -> &mut Node<'static, <M as Model>::Message> {
        if self.view.is_none() {
            unsafe {
                let mut root = (self.model.as_mut() as *mut M).as_mut().unwrap().view();
                root.style(&mut Query::from_style(style));
                self.view.replace(Mutex::new(root));
            }
        }
        self.view.as_mut().unwrap().get_mut().unwrap()
    }
}

impl<M: Model> Drop for ModelView<M> {
    fn drop(&mut self) {
        self.view = None;
    }
}
