use leptos::prelude::*;
use reactive_stores::{Field, Store};

const TERM_WIDTH: u8 = 72;

#[derive(Debug, Store, Clone)]
pub struct TerminalSegment {
    pub id: usize,
    pub text: String,
    pub start_x: u8,
    pub start_y: u32,
}

impl TerminalSegment {
    pub fn new(id: usize, start_x: u8, start_y: u32) -> Self {
        Self {
            id,
            text: String::new(),
            start_x,
            start_y,
        }
    }

    pub fn is_end(&self, cursor_x: u8, cursor_y: u32) -> bool {
        self.start_y == cursor_y && (self.start_x as usize + self.text.len() == cursor_x as usize)
    }

    pub fn append_char(&mut self, c: char) {
        self.text.push(c);
    }
}

#[component]
pub fn TerminalSegment(#[prop(into)] segment: Field<TerminalSegment>) -> impl IntoView {
    view! {
        <span style = move || { format!{"grid-row: {:}; grid-column: {:} / span {:}",
                                        segment.start_y().get() + 1,
                                        segment.start_x().get() + 1,
                                        segment.text().get().len() + 1 } } >
            {move || segment.text().get()}
        </span>
    }
}

#[derive(Debug, Store, Clone)]
pub struct Terminal {
    #[store(key: usize = |segment| segment.id)]
    pub segments: Vec<TerminalSegment>,
    pub cursor_x: u8,
    pub cursor_y: u32,
}

impl Terminal {
    pub fn new() -> Terminal {
        Self {
            segments: vec![TerminalSegment::new(0, 0, 0)],
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_y += 1;
            }
            '\r' => self.cursor_x = 0,
            c => {
                let new_segment = if let Some(last_segment) = self.segments.last_mut() {
                    if last_segment.is_end(self.cursor_x, self.cursor_y) {
                        last_segment.append_char(c);
                        None
                    } else {
                        Some(TerminalSegment::new(
                            self.segments.len(),
                            self.cursor_x,
                            self.cursor_y,
                        ))
                    }
                } else {
                    Some(TerminalSegment::new(
                        self.segments.len(),
                        self.cursor_x,
                        self.cursor_y,
                    ))
                };

                if let Some(mut new_segment) = new_segment {
                    new_segment.append_char(c);
                    self.segments.push(new_segment);
                }

                if self.cursor_x < TERM_WIDTH - 1 {
                    self.cursor_x += 1;
                }
            }
        }
    }

    pub fn push_string(&mut self, str: &str) {
        for c in str.chars() {
            self.write_char(c);
        }
    }
}

#[component]
pub fn Terminal(#[prop(into)] terminal: Field<Terminal>) -> impl IntoView {
    view! {
        <div class = "pane">
            <span class = "terminal">
            <For
                each=move || terminal.segments()
                key=|row| row.id().get()
                children={move |segment| {
                    view! {<TerminalSegment segment  /> }
                }}
            />
            </span>
        </div>
    }
}
