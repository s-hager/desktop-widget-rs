use iced::widget::{column, container, row, text, mouse_area};
use iced::{Element, Length, Alignment, Color};
use crate::stock::StockData;
use crate::components::chart;

#[derive(Debug, Clone)]
pub enum WidgetMessage {
    StartDrag,
}

pub fn view<'a>(data: Option<&'a StockData>, _width: u32, _height: u32) -> Element<'a, WidgetMessage> {
    let content = if let Some(stock) = data {
        let price_color = if stock.change_percent >= 0.0 {
            Color::from_rgb(0.0, 0.8, 0.0)
        } else {
            Color::from_rgb(0.8, 0.0, 0.0)
        };
        
        column![
            row![
                text(&stock.symbol).size(20).color(Color::WHITE),
                text(format!("{:.2}", stock.price)).size(20).color(Color::WHITE),
            ].spacing(10).align_y(Alignment::Center),
            
            text(format!("{:.2}%", stock.change_percent))
                .size(14)
                .color(price_color),
                
            container(Element::from(chart::view(stock)).map(|_| WidgetMessage::StartDrag))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .spacing(5)
    } else {
        column![
            text("Loading...").color(Color::WHITE)
        ]
    };

    let background = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .style(|_theme| {
            container::Style {
                text_color: Some(Color::WHITE),
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.2).into()),
                border: iced::Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    // Wrap in MouseArea for drag
    mouse_area(background)
        .on_press(WidgetMessage::StartDrag)
        .into()
}
