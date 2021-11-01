use super::*;

#[derive(Default)]
pub struct LoginWindow;

pub struct LoginWindowState {
    pub name: String,
    pub password: String,
}

impl Component for LoginWindow {
    type State = LoginWindowState;
    type Message = Message;
    type Output = Message;

    fn mount(&self) -> LoginWindowState {
        LoginWindowState {
            name: "example".to_string(),
            password: "password".to_string(),
        }
    }

    fn view<'a>(&'a self, state: &'a LoginWindowState) -> Node<'a, Message> {
        view! {
            Window() => {
                Row(class="title") => {
                    Text(val="Login window", class="title")
                    Spacer()
                    Spacer(class="close")
                }
                Column() => {
                    Input(
                        placeholder="username",
                        val=state.name.as_str(),
                        on_change=Message::NameChanged,
                        trigger_key=Key::Enter
                    )
                    Input(
                        placeholder="password",
                        val=state.password.as_str(),
                        on_change=Message::PasswordChanged,
                        password=true
                    )
                    Button(text="Login", on_clicked=Message::LoginPressed)
                }
            }
        }
    }

    fn update(&self, message: Message, mut state: State<LoginWindowState>, _: Context<Message, Message>) {
        match message {
            Message::NameChanged(name) => state.name = name,
            Message::PasswordChanged(password) => state.password = password,
            Message::LoginPressed => println!("login pressed!"),
            _ => (),
        }
    }
}
