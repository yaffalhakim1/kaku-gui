use gpui::*;

use crate::theme::Theme;

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
}

impl KakuApp {
    pub fn new(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
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
        })
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
            .child(
                div()
                    .flex_1()
                    .h(px(32.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .child("Type a message..."),
            )
    }
}

impl KakuApp {
    fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
        let label = match status {
            Status::Idle => "Ready",
            Status::Busy => "Thinking…",
            Status::Error(ref e) => e.as_ref(),
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

impl Render for KakuApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let messages = self.messages.clone();
        let status = self.status.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(self.render_messages(messages, theme))
            .child(self.render_input_bar(theme))
            .child(self.render_status_bar(status, theme))
    }
}
