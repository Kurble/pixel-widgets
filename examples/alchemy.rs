use std::collections::HashMap;

use serde::Deserialize;
use winit::window::WindowBuilder;

use pixel_widgets::graphics::Graphics;
use pixel_widgets::node::Node;
use pixel_widgets::prelude::*;
use pixel_widgets::widget::drag_drop::DragDropContext;
use pixel_widgets::widget::panel::Anchor;

pub struct Alchemy {
    items: Vec<Item>,
}

pub struct AlchemyState {
    context: DragDropContext<DragItem>,
    items: Vec<Item>,
    next_item_id: usize,
    playground: Vec<(Item, usize, (f32, f32))>,
}

#[derive(Clone, Copy)]
pub enum DragItem {
    FromInventory(usize),
    FromPlayground(usize),
}

#[derive(Clone)]
pub struct Item {
    id: usize,
    image: Image,
    name: String,
    discovered: bool,
    combinations: HashMap<usize, usize>,
}

#[derive(Clone)]
pub enum Message {
    Place(Item, (f32, f32)),
    MovePlaygroundItem(usize, (f32, f32)),
    CombineInventory(usize, usize),
    CombinePlayground(usize, usize),
}

impl Alchemy {
    fn new(graphics: Graphics) -> Self {
        #[derive(Deserialize)]
        struct Definition {
            image: String,
            name: String,
            unlocked: bool,
            recipe: Option<(String, String)>,
        }

        let defs: Vec<Definition> =
            ron::de::from_bytes(std::fs::read("examples/alchemy/recipes.ron").unwrap().as_slice()).unwrap();

        let mut items: Vec<Item> = defs
            .iter()
            .enumerate()
            .map(|(id, def)| {
                let image = graphics
                    .load_image(std::fs::read(format!("examples/alchemy/{}", def.image)).unwrap())
                    .unwrap();
                Item {
                    id,
                    image,
                    name: def.name.clone(),
                    discovered: def.unlocked,
                    combinations: Default::default(),
                }
            })
            .collect();

        for (index, def) in defs.into_iter().enumerate() {
            if let Some((a, b)) = def.recipe {
                let a = items.iter().position(|i| i.name == a);
                let b = items.iter().position(|i| i.name == b);
                if let (Some(a), Some(b)) = (a, b) {
                    items[a].combinations.insert(b, index);
                    items[b].combinations.insert(a, index);
                }
            }
        }

        Self { items }
    }
}

impl Component for Alchemy {
    type Message = Message;
    type State = AlchemyState;
    type Output = ();

    fn mount(&self) -> Self::State {
        AlchemyState {
            context: Default::default(),
            items: self.items.clone(),
            next_item_id: 0,
            playground: vec![],
        }
    }

    fn view<'a>(&'a self, state: &'a AlchemyState) -> Node<'a, Message> {
        let playground = Layers::with_background(Drop::new(
            &state.context,
            |_| true,
            move |drag_item, pos| match drag_item {
                DragItem::FromInventory(i) => Message::Place(state.items[i].clone(), pos),
                DragItem::FromPlayground(i) => Message::MovePlaygroundItem(i, pos),
            },
            Space,
        ))
        .extend(state.playground.iter().map(|(item, id, pos)| {
            let drag = Drag::new(&state.context, DragItem::FromPlayground(*id), &item.image);
            let drop = Drop::new(
                &state.context,
                |_| true,
                move |drag_item, _| match drag_item {
                    DragItem::FromInventory(other_id) => Message::CombineInventory(*id, other_id),
                    DragItem::FromPlayground(other_id) => Message::CombinePlayground(*id, other_id),
                },
                drag,
            );
            let widget = Panel::new(*pos, Anchor::TopLeft, drop).with_key(id);
            (*id, widget)
        }));

        let filtered: Vec<(usize, &Item)> = state
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.discovered)
            .collect();

        let inventory = Column::new().extend(filtered.chunks(4).map(|row| {
            Row::new().extend(
                row.iter()
                    .map(|&(i, item)| Drag::new(&state.context, DragItem::FromInventory(i), &item.image).with_key(i)),
            )
        }));

        Row::new()
            .push(playground)
            .push(Scroll::new(inventory))
            .with_class("game")
    }

    fn update(
        &self,
        message: Self::Message,
        state: &mut Self::State,
        _: &mut Runtime<Self::Message>,
    ) -> Vec<Self::Output> {
        match message {
            Message::Place(item, pos) => {
                state.playground.push((item, state.next_item_id, pos));
                state.next_item_id += 1;
            }
            Message::MovePlaygroundItem(move_id, new_pos) => {
                for &mut (_, ref id, ref mut pos) in state.playground.iter_mut() {
                    if *id == move_id {
                        *pos = new_pos;
                    }
                }
            }
            Message::CombineInventory(target, item_id) => {
                for &mut (ref mut item, ref id, _) in state.playground.iter_mut() {
                    if *id == target {
                        if let Some(&output) = item.combinations.get(&item_id) {
                            state.items[output].discovered = true;
                            *item = state.items[output].clone();
                        }
                    }
                }
            }
            Message::CombinePlayground(target, source) => {
                if target != source {
                    let mut success = false;
                    let mut item_id = None;
                    for &(ref item, ref id, _) in state.playground.iter() {
                        if *id == source {
                            item_id.replace(item.id);
                        }
                    }
                    if let Some(item_id) = item_id {
                        for &mut (ref mut item, ref id, _) in state.playground.iter_mut() {
                            if *id == target {
                                if let Some(&output) = item.combinations.get(&item_id) {
                                    state.items[output].discovered = true;
                                    *item = state.items[output].clone();
                                    success = true;
                                }
                            }
                        }
                        if success {
                            state.playground.retain(|(_, id, _)| *id != source);
                        }
                    }
                }
            }
        }

        Vec::new()
    }
}

#[tokio::main]
async fn main() {
    let window = WindowBuilder::new()
        .with_title("Alchemy Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 360));

    let style = Style::from_file("examples/alchemy/alchemy.pwss").unwrap();
    let mut sandbox = Sandbox::new(Alchemy::new(style.graphics()), window).await;
    sandbox.ui.set_style(style);

    sandbox.run().await;
}
