use iced::daemon;
use iced::{Subscription, time};
use std::time::Duration;
use tray_icon::{TrayIconEvent, menu::MenuEvent};

mod app;
mod config;
mod stock;
mod components;

use app::{App, Message};

pub fn main() -> iced::Result {
    iced::daemon(App::new, App::update, App::view)
        .title(App::title)
        .subscription(subscription)
        .theme(App::theme)
        .run()
}

fn subscription(_app: &App) -> Subscription<Message> {
    let fast_tick = time::every(Duration::from_millis(200)).map(|_| Message::UnusedTick);
    let slow_tick = time::every(Duration::from_secs(60)).map(|_| Message::Tick);
    
    // We poll events every 200ms
    let event_poller = time::every(Duration::from_millis(200)).map(|_| {
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
             return Message::TrayEvent(event);
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
             return Message::MenuEvent(event);
        }
        Message::None
    });

    Subscription::batch(vec![slow_tick, event_poller])
}
