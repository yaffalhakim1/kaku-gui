use gpui::*;

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: SharedString::default(),
        }
    }

    pub fn content(&self) -> &SharedString {
        &self.content
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content = SharedString::default();
        cx.notify();
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .px(px(8.0))
            .rounded(px(6.0))
            .bg(rgb(0x1e1d26))
            .child(self.content.clone())
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if let Some(c) = event.keystroke.key_char.as_ref() {
                    if c.len() == 1 && !event.keystroke.modifiers.shift {
                        this.content = format!("{}{}", this.content, c).into();
                        cx.notify();
                    }
                }
                if event.keystroke.key == "backspace" {
                    let mut s = this.content.to_string();
                    s.pop();
                    this.content = s.into();
                    cx.notify();
                }
            }))
    }
}
