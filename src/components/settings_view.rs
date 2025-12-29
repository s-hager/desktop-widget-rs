use iced::widget::{button, checkbox, column, row, text, text_input, Column};
use iced::{Element, Length};
use crate::config::{AppConfig, WidgetConfig};

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    ToggleLock(bool),
    Add(String),
    Remove(usize), // Index in list
    InputChanged(String),
}

pub fn view<'a>(config: &'a AppConfig, input_value: &str) -> Element<'a, SettingsMessage> {
    let mut widgets_list = Column::new().spacing(10);

    for (i, widget) in config.widgets.iter().enumerate() {
        widgets_list = widgets_list.push(
            row![
                text(&widget.symbol).width(Length::Fill),
                button("Remove")
                    .on_press(SettingsMessage::Remove(i))
            ]
            .spacing(10)
        );
    }

    column![
        text("Settings").size(30),
        
        row![
            checkbox(config.locked)
                .on_toggle(SettingsMessage::ToggleLock),
            text("Lock Window Positions")
        ].spacing(10),
            
        row![
            text_input("Symbol (e.g. AAPL)", input_value)
                .on_input(SettingsMessage::InputChanged),
            button("Add Widget")
                .on_press(SettingsMessage::Add(input_value.to_string()))
        ].spacing(10),
        
        text("Active Widgets:"),
        widgets_list,
    ]
    .spacing(20)
    .padding(20)
    .into()
}
