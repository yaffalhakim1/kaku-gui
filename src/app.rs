use std::fmt::format;
use std::thread;

use gpui::Decorations::Server;
use gpui::*;

use crate::client::{read_msg, CodexClient, ServerEvent};
use crate::input::TextInput;
use crate::theme::Theme;
use crate::{Abort, SendPrompt};
use serde_json::json;

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
    thread_id: Option<String>,
    client: Option<CodexClient>,
    events: Option<std::sync::mpsc::Receiver<ServerEvent>>,
    streaming_idx: Option<usize>,
    active_turn_id: Option<String>,
    scroll_handle: ScrollHandle,
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
                thread_id: None,
                client: None,
                events: None,
                streaming_idx: None,
                active_turn_id: None,
                scroll_handle: ScrollHandle::new(),
            }
        });
        app.update(cx, |this, cx| this.connect(cx));
        app
    }
}

impl KakuApp {
    fn connect(&mut self, cx: &mut Context<Self>) {
        let (tx, rx) = std::sync::mpsc::channel::<ServerEvent>();
        self.events = Some(rx);

        cx.background_executor()
            .spawn(async move {
                let (client, mut stdout) = match CodexClient::spawn() {
                    Ok(pair) => pair,
                    Err(e) => {
                        let _ = tx.send(ServerEvent::Disconnected(format!("connect: {e:#}")));
                        return;
                    }
                };

                let cwd = match std::env::current_dir() {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = tx.send(ServerEvent::Disconnected(format!("cwd: {e:#}")));
                        return;
                    }
                };

                let send = client.request("thread/start", json!({ "cwd": cwd }));
                if let Err(e) = send {
                    let _ = tx.send(ServerEvent::Disconnected(format!("thread/start: {e:#}")));
                    return;
                }

                let _ = tx.send(ServerEvent::Ready { client });

                loop {
                    match read_msg(&mut stdout) {
                        Ok(Some(msg)) => {
                            if tx.send(classify(msg)).is_err() {
                                return; // receiver dropped: app closing
                            }
                        }
                        Ok(None) => {
                            let _ = tx.send(ServerEvent::Disconnected("codex exited".to_string()));
                            return;
                        }
                        Err(e) => {
                            let _ = tx.send(ServerEvent::Disconnected(format!("{e:#}")));
                            return;
                        }
                    }
                }
            })
            .detach();
    }

    fn apply_event(&mut self, event: ServerEvent, cx: &mut Context<Self>) {
        match event {
            ServerEvent::Ready { client } => self.client = Some(client),
            ServerEvent::ThreadStarted { id } => {
                self.thread_id = Some(id);
                self.status = Status::Idle;
            }
            ServerEvent::Disconnected(why) => {
                self.status = Status::Error(format!("disconnected: {why}"));
            }
            ServerEvent::TurnStarted { turn_id } => {
                self.active_turn_id = Some(turn_id);
            }
            ServerEvent::AgentMessageDelta { delta } => {
                if let Some(idx) = self.streaming_idx {
                    if let Some(message) = self.messages.get_mut(idx) {
                        message.text.push_str(&delta);
                        self.scroll_handle.scroll_to_bottom();
                    }
                }
            }
            ServerEvent::TurnCompleted => {
                self.status = Status::Idle;
                self.streaming_idx = None;
                self.active_turn_id = None;
            }

            _ => {}
        }
        cx.notify();
    }
}

impl KakuApp {
    fn render_messages(&self, messages: Vec<DisplayMessage>, theme: Theme) -> impl IntoElement {
        div()
            .id("messages")
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(16.0))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
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

        let Some(client) = self.client.clone() else {
            self.status = Status::Error("not connected".to_string());
            cx.notify();
            return;
        };
        let Some(thread_id) = self.thread_id.clone() else {
            return;
        };

        self.messages.push(DisplayMessage {
            role: Role::User,
            text: text.clone(),
        });
        self.messages.push(DisplayMessage {
            role: Role::Assistant,
            text: String::new(),
        });
        self.streaming_idx = Some(self.messages.len() - 1);
        self.status = Status::Busy;
        self.input.update(cx, |input, cx| input.clear(cx));
        cx.notify();

        if let Err(e) = client.request(
            "turn/start",
            json!({
                "threadId":thread_id,
                "input":[{"type" : "text", "text":text}],
            }),
        ) {
            self.status = Status::Error(format!("send: {e:#}"));
            cx.notify();
        }
    }

    fn abort_action(&mut self, _: &Abort, _window: &mut Window, cx: &mut Context<Self>) {
        let (Some(client), Some(thread_id), Some(turn_id)) = (
            self.client.as_ref(),
            self.thread_id.as_ref(),
            self.active_turn_id.as_ref(),
        ) else {
            return;
        };

        let thread_id = thread_id.clone();
        let turn_id = turn_id.clone();
        if let Err(e) = client.interrupt(&thread_id, &turn_id) {
            self.status = Status::Error(format!("abort: {e:#}"));
        }
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
        let thread_id = self
            .thread_id
            .clone()
            .unwrap_or_else(|| "not connected".to_string());

        let (label, dot_color) = match status {
            Status::Idle => (format!("Ready — {thread_id}"), theme.accent),
            Status::Busy => ("Thinking…".to_string(), theme.text),
            Status::Error(ref e) => (format!("Error: {e}"), theme.user),
        };

        div()
            .h(px(24.0))
            .px(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .border_t_1()
            .border_color(theme.border)
            .child(div().size(px(6.0)).rounded_full().bg(dot_color))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted)
                    .child(label),
            )
    }
}

impl Focusable for KakuApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn classify(msg: serde_json::Value) -> ServerEvent {
    let Some(method) = msg.get("method").and_then(|m| m.as_str()) else {
        return ServerEvent::Ignored; // a response (has id, no method)
    };
    let params = &msg["params"];

    match method {
        "thread/started" => ServerEvent::ThreadStarted {
            id: params["thread"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        },
        "turn/started" => ServerEvent::TurnStarted {
            turn_id: params["turn"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        },
        "item/agentMessage/delta" => ServerEvent::AgentMessageDelta {
            delta: params["delta"].as_str().unwrap_or_default().to_string(),
        },
        "item/completed" => ServerEvent::ItemCompleted,
        "turn/completed" => ServerEvent::TurnCompleted,
        _ => ServerEvent::Ignored,
    }
}

impl Render for KakuApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while let Some(event) = self.events.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.apply_event(event, cx);
        }

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
            .on_action(cx.listener(Self::abort_action))
            .child(self.render_messages(messages, theme))
            .child(self.render_input_bar(theme))
            .child(self.render_status_bar(status, theme))
    }
}
