use pixel_widgets::loader::Loader;
use pixel_widgets::prelude::*;
use pixel_widgets::widget::drag_drop::DragDropContext;
use pixel_widgets::widget::panel::Anchor;
use pixel_widgets::Command;
use serde::Deserialize;
use std::collections::HashMap;
use winit::window::WindowBuilder;

enum Alchemy {
    Loading {
        progress: usize,
        total: usize,
    },
    Game {
        state: ManagedState<Id>,
        context: DragDropContext<DragItem>,
        items: Vec<Item>,
        next_item_id: usize,
        playground: Vec<(Item, usize, (f32, f32))>,
    },
}

#[derive(Clone, Copy)]
enum DragItem {
    FromInventory(usize),
    FromPlayground(usize),
}

#[derive(PartialEq, Eq, Clone)]
enum Id {
    Inventory,
    InventoryItem(usize),
    Playground,
    PlaygroundItem(usize),
}

#[derive(Clone)]
struct Item {
    id: usize,
    image: Image,
    name: String,
    discovered: bool,
    combinations: HashMap<usize, usize>,
}

#[derive(Clone)]
enum Message {
    Void,
    LoadItems(usize),
    LoadedItem,
    Loaded(Vec<Item>),
    Place(Item, (f32, f32)),
    MovePlaygroundItem(usize, (f32, f32)),
    CombineInventory(usize, usize),
    CombinePlayground(usize, usize),
}

impl UpdateModel for Alchemy {
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Vec<Command<Message>> {
        match message {
            Message::Void => (),
            Message::LoadItems(total) => {
                *self = Self::Loading { progress: 0, total };
            }
            Message::LoadedItem => {
                if let Self::Loading { ref mut progress, .. } = *self {
                    *progress += 1;
                }
            }
            Message::Loaded(items) => {
                *self = Self::Game {
                    state: Default::default(),
                    context: Default::default(),
                    items,
                    next_item_id: 0,
                    playground: Default::default(),
                };
            }
            Message::Place(item, pos) => {
                if let Self::Game {
                    ref mut playground,
                    ref mut next_item_id,
                    ..
                } = *self
                {
                    playground.push((item, *next_item_id, pos));
                    *next_item_id += 1;
                }
            }
            Message::MovePlaygroundItem(move_id, new_pos) => {
                if let Self::Game { ref mut playground, .. } = *self {
                    for &mut (_, ref id, ref mut pos) in playground.iter_mut() {
                        if *id == move_id {
                            *pos = new_pos;
                        }
                    }
                }
            }
            Message::CombineInventory(target, item_id) => {
                if let Self::Game {
                    ref mut playground,
                    ref mut items,
                    ..
                } = *self
                {
                    for &mut (ref mut item, ref id, _) in playground.iter_mut() {
                        if *id == target {
                            if let Some(&output) = item.combinations.get(&item_id) {
                                items[output].discovered = true;
                                *item = items[output].clone();
                            }
                        }
                    }
                }
            }
            Message::CombinePlayground(target, source) => {
                if let Self::Game {
                    ref mut playground,
                    ref mut items,
                    ..
                } = *self
                {
                    if target != source {
                        let mut success = false;
                        let mut item_id = None;
                        for &(ref item, ref id, _) in playground.iter() {
                            if *id == source {
                                item_id.replace(item.id);
                            }
                        }
                        if let Some(item_id) = item_id {
                            for &mut (ref mut item, ref id, _) in playground.iter_mut() {
                                if *id == target {
                                    if let Some(&output) = item.combinations.get(&item_id) {
                                        items[output].discovered = true;
                                        *item = items[output].clone();
                                        success = true;
                                    }
                                }
                            }
                            if success {
                                playground.retain(|(_, id, _)| *id != source);
                            }
                        }
                    }
                }
            }
        }

        Vec::new()
    }

    fn view(&mut self) -> Node<Message> {
        match self {
            &mut Self::Loading { progress, total } => Column::new()
                .push(Space)
                .push(Progress::new(progress as f32 / total as f32))
                .class("loading"),
            &mut Self::Game {
                ref mut state,
                ref context,
                ref items,
                ref playground,
                ..
            } => {
                let mut state = state.tracker();

                let playground = Layers::with_background(
                    state.get(&Id::Playground),
                    Drop::new(
                        state.get(&Id::Playground),
                        context,
                        |_| true,
                        move |drag_item, pos| match drag_item {
                            DragItem::FromInventory(i) => Message::Place(items[i].clone(), pos),
                            DragItem::FromPlayground(i) => Message::MovePlaygroundItem(i, pos),
                        },
                        Space,
                    ),
                )
                .extend(playground.iter().map(|(item, id, pos)| {
                    let drag = Drag::new(
                        state.get(&Id::PlaygroundItem(*id)),
                        context,
                        DragItem::FromPlayground(*id),
                        &item.image,
                    );
                    let drop = Drop::new(
                        state.get(&Id::PlaygroundItem(*id)),
                        context,
                        |_| true,
                        move |drag_item, _| match drag_item {
                            DragItem::FromInventory(other_id) => Message::CombineInventory(*id, other_id),
                            DragItem::FromPlayground(other_id) => Message::CombinePlayground(*id, other_id),
                        },
                        drag,
                    );
                    let widget = Panel::new(*pos, Anchor::TopLeft, drop);
                    (*id, widget)
                }));

                let filtered: Vec<(usize, &Item)> =
                    items.iter().enumerate().filter(|(_, item)| item.discovered).collect();

                let inventory = Column::new().extend(filtered.chunks(4).map(|row| {
                    Row::new().extend(row.iter().map(|&(i, item)| {
                        Drag::new(
                            state.get(&Id::InventoryItem(i)),
                            context,
                            DragItem::FromInventory(i),
                            &item.image,
                        )
                    }))
                }));

                Row::new()
                    .push(playground)
                    .push(Scroll::new(state.get(&Id::Inventory), inventory))
                    .class("game")
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let model = Alchemy::Loading { progress: 0, total: 1 };

    let window = WindowBuilder::new()
        .with_title("Alchemy Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 360));

    let loader = pixel_widgets::loader::FsLoader::new("./examples/alchemy".into()).unwrap();

    let mut sandbox = Sandbox::new(model, loader, window).await;
    sandbox.ui.set_stylesheet("alchemy.pwss").await.unwrap();

    let graphics = sandbox.ui.graphics();
    let (tx, rx) = futures::channel::mpsc::unbounded();
    sandbox.ui.command(Command::from_stream(rx));
    sandbox.ui.command(Command::from_future_message(async move {
        let graphics = graphics.clone();
        tx.unbounded_send(Message::LoadItems(1)).ok();

        #[derive(Deserialize)]
        struct Definition {
            image: String,
            name: String,
            unlocked: bool,
            recipe: Option<(String, String)>,
        }

        let bytes = graphics.loader().load("recipes.ron").await.unwrap();
        let defs: Vec<Definition> = ron::de::from_bytes(bytes.as_slice()).unwrap();
        tx.unbounded_send(Message::LoadItems(defs.len() + 1)).ok();
        tx.unbounded_send(Message::LoadedItem).ok();

        let mut items = futures::future::join_all(defs.iter().enumerate().map(|(id, def)| {
            let graphics = graphics.clone();
            let tx = tx.clone();
            async move {
                let image = graphics.load_image(def.image.as_str()).await.unwrap();
                tx.unbounded_send(Message::LoadedItem).ok();
                Item {
                    id,
                    image,
                    name: def.name.clone(),
                    discovered: def.unlocked,
                    combinations: HashMap::new(),
                }
            }
        }))
        .await;

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

        tx.unbounded_send(Message::Loaded(items)).ok();
        Message::Void
    }));

    sandbox.run().await;
}
