use winit::window::WindowBuilder;

use pixel_widgets::node::Node;
use pixel_widgets::prelude::*;

struct Counter;

#[derive(Clone)]
enum Message {
    UpPressed,
    DownPressed,
}

macro_rules! ui {
    { $w:expr $(=>{ $(
        $(;for $x:ident in $i:expr =>)?
        $(;if $(let $y_pat:pat =)? $y:expr =>)?
        $child:expr $(=>$sub_child:tt)?
        $(;else => $else_child:expr $(=>$else_sub_child:tt)?)?
        $(;else if $(let $z_pat:pat =)? $z:expr => $else_if_child:expr $(=>$else_if_sub_child:tt)?)?
    )* })? } => {
        $w$($(.extend(ui!{ inner
            $(;for $x in $i =>)?
            $(;if $(let $y_pat =)? $y =>)?
            $child $(=>$sub_child)?
            $(;else =>)?
            $(;else if $(let $z_pat =)? $z =>)?
        }))*)?.into_node()
    };

    { inner ;for $x:ident in $i:expr => $w:expr $(=>$cs:tt)? } => {
        $i.into_iter().map(|$x| ui!{$w $(=>$cs)?})
    };
    { inner ;if $(let $y_pat:pat =)? $y:expr => $w:expr $(=>$cs:tt)? } => {
        if $(let $y_pat =)? $y {
            Some(ui!{$w $(=>$cs)?})
        } else {
            None
        }
    };
    { inner ;if $(let $y_pat:pat =)? $y:expr => $w:expr $(=>$cs:tt)? ;else => } => {
        if $(let $y_pat =)? $y {
            Some(ui!{$w $(=>$cs)?})
        } else {
            Some(ui!{$else_w $(=>$else_cs)?})
        }
    };
    { inner ;if $(let $y_pat:pat =)? $y:expr => $w:expr $(=>$cs:tt)? ;else if $($z_pat:pat =)? $z:expr => $else_w:expr $(=>$else_cs:tt)? } => {
        if $(let $y_pat =)? $y {
            Some(ui!{$w $(=>$cs)?})
        } else {
            Some(ui!{$else_w $(=>$else_cs)?})
        }
    };
    { inner $w:expr $(=>$cs:tt)? } => {
        Some(ui!{$w $(=>$cs)?})
    };
}

impl Component for Counter {
    type Message = Message;
    type State = i32;
    type Output = ();

    fn mount(&self) -> Self::State {
        15
    }

    fn view(&self, state: &i32) -> Node<Message> {
        let mut test = Some("pattern");
        test.take();
        ui! {
            Column::new() => {
                Button::new("Up")
                    .on_clicked(Message::UpPressed)

                Column::new() => {
                    ;for x in ["a", "b", "c"].iter() => x
                    ;if let Some(x) = test => x
                    ;else => "lolno"
                    //:else if false => Row::new()
                    //:else => "hoi"
                }

                format!("Count: {}", *state)

                Button::new("Down")
                    .on_clicked(Message::DownPressed)
            }
        }
    }

    fn update(
        &self,
        message: Self::Message,
        state: &mut i32,
        _runtime: &mut Runtime<Self::Message>,
    ) -> Vec<Self::Output> {
        match message {
            Message::UpPressed => {
                *state += 1;
                Vec::new()
            }
            Message::DownPressed => {
                *state -= 1;
                Vec::new()
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let window = WindowBuilder::new()
        .with_title("Counter")
        .with_inner_size(winit::dpi::LogicalSize::new(240, 240));

    let mut sandbox = Sandbox::new(Counter, window).await;
    sandbox.ui.set_style(Style::from_file("examples/counter.pwss").unwrap());

    sandbox.run().await;
}
