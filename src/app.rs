use gpui::*;

use crate::input::TextInput;
use crate::theme::Theme;
use crate::SendPrompt;

use crate::client::{OpencodeClient, Session};
use gpui::http_client::Url;

#[derive(Debug, Clone)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug)]
pub struct DisplayMessage {
    pub role: Role,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum Status {
    Idle,
    Busy,
    Error(String),
}

pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
    status: Status,
    input: Entity<TextInput>,
    session: Option<Session>,
    client: Option<OpencodeClient>,
}

impl KakuApp {
    pub fn new(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        let app = cx.new(|cx| {
            let input = cx.new(|cx| TextInput::new(cx));
            Self {
                focus_handle: cx.focus_handle(),
                theme: Theme::dark(),
                messages: vec![
                    DisplayMessage {
                        role: Role::User,
                        text: "Hello, who are you?".to_string(),
                    },
                    DisplayMessage {
                        role: Role::Assistant,
                        text: "I am an AI assistant running inside a native GPUI app.".to_string(),
                    },
                ],
                status: Status::Idle,
                input,
                session: None,
                client: None,
            }
        });
        app.update(cx, |this, cx| this.connect(cx));
        app
    }
}

impl KakuApp {
    fn render_messages(&self, messages: Vec<DisplayMessage>, theme: Theme) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(16.0))
            .children(
                messages
                    .into_iter()
                    .map(move |m| self.render_message(m, theme)),
            )
    }
}

impl KakuApp {
    fn render_message(&self, m: DisplayMessage, theme: Theme) -> impl IntoElement {
        let (prefix, color) = match m.role {
            Role::User => ("> ", theme.user),
            Role::Assistant => ("", theme.text),
            Role::System => ("", theme.muted),
        };

        div()
            .flex()
            .flex_row()
            .gap(px(4.0))
            .child(div().text_color(color).child(prefix.to_string()))
            .child(div().text_color(theme.text).child(m.text))
    }
}

impl KakuApp {
    fn send_prompt_action(&mut self, _: &SendPrompt, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.input.read(cx).content().clone();
        let text = content.to_string();
        if text.trim().is_empty() {
            return;
        }
        self.messages.push(DisplayMessage {
            role: Role::User,
            text: text.clone(),
        });
        self.input.update(cx, |input, cx| input.clear(cx));
        cx.notify();
    }
}

impl KakuApp {
    fn render_input_bar(&self, theme: Theme) -> impl IntoElement {
        div()
            .h(px(48.0))
            .px(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .border_t_1()
            .border_color(theme.border)
            .child(div().text_color(theme.accent).child("› "))
            .child(self.input.clone())
    }
}

impl KakuApp {
    fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
        let session_id = self
            .session
            .as_ref()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "not connected".to_string());

        let label = match status {
            Status::Idle => format!("Ready — {session_id}"),
            Status::Busy => "Thinking…".to_string(),
            Status::Error(ref e) => format!("Error: {e}"),
        };

        div()
            .h(px(24.0))
            .px(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted)
                    .child(label.to_string()),
            )
    }
}

impl Focusable for KakuApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl KakuApp {
    fn connect(&mut self, cx: &mut Context<Self>) {
        let base =
            std::env::var("KAKU_GUI_URL").unwrap_or_else(|_| "http://127.0.0.1:4096".to_string());

        let url = match Url::parse(&base) {
            Ok(u) => u,
            Err(e) => {
                self.status = Status::Error(format!("Invalid URL: {e}"));
                cx.notify();
                return;
            }
        };

        let client = OpencodeClient::new(url, cx.http_client());

        cx.spawn(async move |this, cx| {
            let result = async {
                client.health().await?;
                let session = client.create_session("kaku-gui").await?;
                Ok::<_, anyhow::Error>((client, session))
            }
            .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((client, session)) => {
                        this.client = Some(client);
                        this.session = Some(session);
                        this.status = Status::Idle;
                    }
                    Err(e) => {
                        this.status = Status::Error(format!("connect: {e:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for KakuApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let messages = self.messages.clone();
        let status = self.status.clone();

        div()
            .key_context("KakuApp")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .on_action(cx.listener(Self::send_prompt_action))
            .child(self.render_messages(messages, theme))
            .child(self.render_input_bar(theme))
            .child(self.render_status_bar(status, theme))
    }
}
