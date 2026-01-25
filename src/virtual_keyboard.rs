use iced::widget::{button, container, text, Column, Row};
use iced::{Alignment, Element, Length};

use crate::ui_theme;

#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardMessage {
    Press(usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardOutput {
    None,
    Input(String),
    Submit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LayoutType {
    Qwerty,
    Symbols,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum KeyType {
    Char(char),
    Shift,
    LayoutToggle,
    Space,
    Backspace,
    Submit,
}

#[derive(Debug, Clone, Copy)]
struct KeyDef {
    label: &'static str,
    key_type: KeyType,
    width_units: u16,
}

impl KeyDef {
    const fn char(c: char) -> Self {
        Self {
            label: "",
            key_type: KeyType::Char(c),
            width_units: 1,
        }
    }

    const fn special(label: &'static str, key_type: KeyType, width_units: u16) -> Self {
        Self {
            label,
            key_type,
            width_units,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VirtualKeyboard {
    value: String,
    cursor: (usize, usize),
    layout: LayoutType,
    shift: bool,
    max_length: Option<usize>,
    password_mode: bool,
}

impl VirtualKeyboard {
    pub fn new(initial_value: String) -> Self {
        Self {
            value: initial_value,
            cursor: (1, 0),
            layout: LayoutType::Qwerty,
            shift: false,
            max_length: None,
            password_mode: false,
        }
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn password(mut self) -> Self {
        self.password_mode = true;
        self
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }

    pub fn display_value(&self) -> String {
        if self.password_mode {
            "*".repeat(self.value.len())
        } else {
            self.value.clone()
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            self.clamp_cursor();
        }
    }

    pub fn move_down(&mut self) {
        let rows = self.current_layout();
        if self.cursor.0 + 1 < rows.len() {
            self.cursor.0 += 1;
            self.clamp_cursor();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
        }
    }

    pub fn move_right(&mut self) {
        let rows = self.current_layout();
        if let Some(row) = rows.get(self.cursor.0) {
            if self.cursor.1 + 1 < row.len() {
                self.cursor.1 += 1;
            }
        }
    }

    pub fn select_current(&mut self) -> KeyboardOutput {
        let rows = self.current_layout();
        if let Some(row) = rows.get(self.cursor.0) {
            if let Some(key) = row.get(self.cursor.1) {
                return self.handle_key_press(*key);
            }
        }
        KeyboardOutput::None
    }

    pub fn backspace(&mut self) -> KeyboardOutput {
        if !self.value.is_empty() {
            self.value.pop();
            return KeyboardOutput::Input(self.value.clone());
        }
        KeyboardOutput::None
    }

    pub fn handle_message(&mut self, message: KeyboardMessage) -> KeyboardOutput {
        match message {
            KeyboardMessage::Press(row, col) => {
                self.cursor = (row, col);
                self.select_current()
            }
        }
    }

    pub fn view(&self, scale: f32) -> Element<'_, KeyboardMessage> {
        let mut content = Column::new()
            .spacing(ui_theme::scaled(ui_theme::ITEM_SPACING, scale))
            .align_x(Alignment::Center);

        let key_size = ui_theme::scaled(42.0, scale);
        let key_height = ui_theme::scaled(42.0, scale);
        let key_spacing = ui_theme::scaled(ui_theme::ITEM_SPACING, scale);

        for (row_index, row_keys) in self.current_layout().iter().enumerate() {
            let mut row_widget = Row::new().spacing(key_spacing).align_y(Alignment::Center);

            for (col_index, key) in row_keys.iter().enumerate() {
                let is_selected = self.cursor == (row_index, col_index);
                let label = self.key_label(*key);
                let width =
                    key_size * key.width_units as f32 + key_spacing * (key.width_units - 1) as f32;

                let label_text = text(label)
                    .font(ui_theme::SANSATION)
                    .size(ui_theme::scaled(ui_theme::BASE_FONT_LARGE, scale))
                    .color(if is_selected {
                        ui_theme::COLOR_ABYSS_DARK
                    } else {
                        ui_theme::COLOR_TEXT_BRIGHT
                    })
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center);

                let tile = container(label_text)
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(key_height))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(move |_| iced::widget::container::Style {
                        background: Some(
                            if is_selected {
                                ui_theme::COLOR_ACCENT
                            } else {
                                ui_theme::COLOR_PANEL
                            }
                            .into(),
                        ),
                        border: iced::Border {
                            color: ui_theme::COLOR_TEXT_MUTED,
                            width: ui_theme::scaled(1.0, scale),
                            radius: ui_theme::scaled(6.0, scale).into(),
                        },
                        ..Default::default()
                    });

                let tile = button(tile)
                    .padding(0)
                    .style(|_, _| button::Style::default())
                    .on_press(KeyboardMessage::Press(row_index, col_index));

                row_widget = row_widget.push(tile);
            }

            content = content.push(row_widget);
        }

        content.into()
    }

    fn clamp_cursor(&mut self) {
        let rows = self.current_layout();
        if self.cursor.0 >= rows.len() {
            self.cursor.0 = rows.len().saturating_sub(1);
        }
        if let Some(row) = rows.get(self.cursor.0) {
            if self.cursor.1 >= row.len() {
                self.cursor.1 = row.len().saturating_sub(1);
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyDef) -> KeyboardOutput {
        match key.key_type {
            KeyType::Char(c) => {
                if let Some(max) = self.max_length {
                    if self.value.len() >= max {
                        return KeyboardOutput::None;
                    }
                }
                let ch = if self.shift {
                    c.to_ascii_uppercase()
                } else {
                    c
                };
                self.value.push(ch);
                KeyboardOutput::Input(self.value.clone())
            }
            KeyType::Space => {
                if let Some(max) = self.max_length {
                    if self.value.len() >= max {
                        return KeyboardOutput::None;
                    }
                }
                self.value.push(' ');
                KeyboardOutput::Input(self.value.clone())
            }
            KeyType::Backspace => self.backspace(),
            KeyType::Submit => KeyboardOutput::Submit,
            KeyType::Shift => {
                self.shift = !self.shift;
                KeyboardOutput::None
            }
            KeyType::LayoutToggle => {
                self.layout = match self.layout {
                    LayoutType::Qwerty => LayoutType::Symbols,
                    LayoutType::Symbols => LayoutType::Qwerty,
                };
                self.clamp_cursor();
                KeyboardOutput::None
            }
        }
    }

    fn key_label(&self, key: KeyDef) -> String {
        match key.key_type {
            KeyType::Char(c) => {
                if self.shift {
                    c.to_ascii_uppercase().to_string()
                } else {
                    c.to_string()
                }
            }
            KeyType::LayoutToggle => match self.layout {
                LayoutType::Qwerty => "Sym".to_string(),
                LayoutType::Symbols => "ABC".to_string(),
            },
            KeyType::Shift => {
                if self.shift {
                    "SHIFT".to_string()
                } else {
                    "Shift".to_string()
                }
            }
            _ => key.label.to_string(),
        }
    }

    fn current_layout(&self) -> &'static [&'static [KeyDef]] {
        match self.layout {
            LayoutType::Qwerty => QWERTY_LAYOUT,
            LayoutType::Symbols => SYMBOLS_LAYOUT,
        }
    }
}

const QWERTY_LAYOUT: &[&[KeyDef]] = &[
    &[
        KeyDef::char('1'),
        KeyDef::char('2'),
        KeyDef::char('3'),
        KeyDef::char('4'),
        KeyDef::char('5'),
        KeyDef::char('6'),
        KeyDef::char('7'),
        KeyDef::char('8'),
        KeyDef::char('9'),
        KeyDef::char('0'),
    ],
    &[
        KeyDef::char('q'),
        KeyDef::char('w'),
        KeyDef::char('e'),
        KeyDef::char('r'),
        KeyDef::char('t'),
        KeyDef::char('y'),
        KeyDef::char('u'),
        KeyDef::char('i'),
        KeyDef::char('o'),
        KeyDef::char('p'),
    ],
    &[
        KeyDef::char('a'),
        KeyDef::char('s'),
        KeyDef::char('d'),
        KeyDef::char('f'),
        KeyDef::char('g'),
        KeyDef::char('h'),
        KeyDef::char('j'),
        KeyDef::char('k'),
        KeyDef::char('l'),
    ],
    &[
        KeyDef::special("Shift", KeyType::Shift, 2),
        KeyDef::char('z'),
        KeyDef::char('x'),
        KeyDef::char('c'),
        KeyDef::char('v'),
        KeyDef::char('b'),
        KeyDef::char('n'),
        KeyDef::char('m'),
        KeyDef::special("Bksp", KeyType::Backspace, 2),
    ],
    &[
        KeyDef::special("Sym", KeyType::LayoutToggle, 2),
        KeyDef::special("Space", KeyType::Space, 5),
        KeyDef::special("OK", KeyType::Submit, 2),
    ],
];

const SYMBOLS_LAYOUT: &[&[KeyDef]] = &[
    &[
        KeyDef::char('1'),
        KeyDef::char('2'),
        KeyDef::char('3'),
        KeyDef::char('4'),
        KeyDef::char('5'),
        KeyDef::char('6'),
        KeyDef::char('7'),
        KeyDef::char('8'),
        KeyDef::char('9'),
        KeyDef::char('0'),
    ],
    &[
        KeyDef::char('!'),
        KeyDef::char('@'),
        KeyDef::char('#'),
        KeyDef::char('$'),
        KeyDef::char('%'),
        KeyDef::char('^'),
        KeyDef::char('&'),
        KeyDef::char('*'),
        KeyDef::char('('),
        KeyDef::char(')'),
    ],
    &[
        KeyDef::char('-'),
        KeyDef::char('_'),
        KeyDef::char('='),
        KeyDef::char('+'),
        KeyDef::char('['),
        KeyDef::char(']'),
        KeyDef::char('{'),
        KeyDef::char('}'),
        KeyDef::char('\\'),
    ],
    &[
        KeyDef::special("Shift", KeyType::Shift, 2),
        KeyDef::char(';'),
        KeyDef::char(':'),
        KeyDef::char('\''),
        KeyDef::char('"'),
        KeyDef::char(','),
        KeyDef::char('.'),
        KeyDef::char('?'),
        KeyDef::special("Bksp", KeyType::Backspace, 2),
    ],
    &[
        KeyDef::special("ABC", KeyType::LayoutToggle, 2),
        KeyDef::special("Space", KeyType::Space, 5),
        KeyDef::special("OK", KeyType::Submit, 2),
    ],
];
