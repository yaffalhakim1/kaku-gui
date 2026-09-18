use gpui::*;

/// kaku color palette ported from kaku-tui/src/theme.rs.
/// All colors are RGB so GPUI's opacity modifiers work later.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Hsla,
    pub surface: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub user: Hsla,
    pub accent: Hsla,
    pub border: Hsla,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: rgb(0x15141b).into(),
            surface: rgb(0x1e1d26).into(),
            text: rgb(0xd5d4d6).into(),
            muted: rgb(0x6d6d6d).into(),
            user: rgb(0x8e6ad9).into(),
            accent: rgb(0xdaae76).into(),
            border: rgb(0x33323a).into(),
        }
    }
}
