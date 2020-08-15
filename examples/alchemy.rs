use pixel_widgets::prelude::*;
use winit::window::WindowBuilder;
use pixel_widgets::Command;
use pixel_widgets::loader::Loader;
use std::collections::HashMap;
use serde::Deserialize;
use pixel_widgets::widget::drag_drop::DragDropContext;

enum Alchemy {
    Loading {
        progress: usize,
        total: usize,
    },
    Game {
        state: ManagedState<Id>,
        context: DragDropContext<usize>,
        items: Vec<Item>,
        // todo
    },
}

#[derive(PartialEq, Eq, Clone)]
enum Id {
    Inventory,
    InventoryItem(usize),
}

#[derive(Clone)]
struct Item {
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
}

impl Model for Alchemy {
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
                    context: DragDropContext::default(),
                    items,
                };
            }
        }

        Vec::new()
    }

    fn view(&mut self) -> Node<Message> {
        match self {
            &mut Self::Loading { progress, total } => {
                Column::new()
                    .push(Space)
                    .push(Progress::new(progress as f32 / total as f32))
                    .class("loading")
            }
            &mut Self::Game { ref mut state, ref context, ref items } => {
                let mut state = state.tracker();

                let filtered: Vec<(usize, &Item)> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.discovered)
                    .collect();

                let inventory = Column::new()
                    .extend(filtered.chunks(4).map(|row| {
                        Row::new()
                            .extend(row.iter().map(|&(i, item)| {
                                Drag::new(state.get(&Id::InventoryItem(i)), context, i, &item.image)
                            }))
                    }));

                Row::new()
                    .push(Space)
                    .push(Scroll::new(state.get(&Id::Inventory), inventory))
                    .class("game")
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let model = Alchemy::Loading {
        progress: 0,
        total: 1,
    };

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

        let mut items = futures::future::join_all(defs.iter().map(|def| {
            let graphics = graphics.clone();
            let tx = tx.clone();
            async move {
                let image = graphics.load_image(def.image.as_str()).await.unwrap();
                tx.unbounded_send(Message::LoadedItem).ok();
                Item {
                    image,
                    name: def.name.clone(),
                    discovered: def.unlocked,
                    combinations: HashMap::new(),
                }
            }
        })).await;

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
