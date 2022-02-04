use super::*;

#[derive(Default)]
pub struct LoginWindow;

pub enum LoginWindowState {
    Prompt { name: String, password: String },
    Busy,
}

impl Component for LoginWindow {
    type State = LoginWindowState;
    type Message = Message;
    type Output = Message;

    fn mount(&self, _: &mut Runtime<Message>) -> LoginWindowState {
        LoginWindowState::Prompt {
            name: "example".to_string(),
            password: "password".to_string(),
        }
    }

    fn view<'a>(&'a self, state: &'a LoginWindowState) -> Node<'a, Message> {
        view! {
            Window => {
                Row { class: "title" } => {
                    Text { val: "Login window", class: "title" }
                    Spacer
                    Spacer { class: "close" }
                }

                [match state]
                [case LoginWindowState::Prompt { name, password }]
                Column => {
                    Input {
                        placeholder: "username",
                        val: name.as_str(),
                        on_change: Message::NameChanged,
                        trigger_key: Key::Enter
                    }
                    Input {
                        placeholder: "password",
                        val: password.as_str(),
                        on_change: Message::PasswordChanged,
                        password: true
                    }
                    Button { text: "Login", on_clicked: Message::LoginPressed }
                }
                [case LoginWindowState::Busy] 
                Column => {
                    Text { val: "logging in!" }
                }
            }
        }
    }

    fn style() -> StyleBuilder {
        StyleBuilder::default().rule(RuleBuilder::new("window").background_color(Color::rgb(0.3, 0.3, 0.5)))
    }

    fn update(
        &self,
        message: Message,
        mut state: DetectMut<LoginWindowState>,
        _: &mut Runtime<Message>,
        _: &mut Context<Message>,
    ) {
        match message {
            Message::NameChanged(new_name) => {
                if let LoginWindowState::Prompt { name, .. } = &mut *state {
                    *name = new_name;
                }
            }
            Message::PasswordChanged(new_password) => {
                if let LoginWindowState::Prompt { password, .. } = &mut *state {
                    *password = new_password;
                }
            }
            Message::LoginPressed => *state = LoginWindowState::Busy,
            _ => (),
        }
    }
}
