use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, text, text_input, vertical_space, horizontal_rule, tooltip, svg};
use iced::{Element, Length, Theme, Command, Application, Settings, Subscription, Alignment};
use crate::ipc::{IpcMessage, ChartData, ConfigData, PIPE_NAME};
use crate::language::{self, TextId};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn run() -> iced::Result {
    SettingsApp::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(450.0, 650.0),
            min_size: Some(iced::Size::new(450.0, 600.0)),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

struct SettingsApp {
    charts: Vec<ChartData>,
    config: Option<ConfigData>,
    update_status: Option<crate::common::UpdateStatus>,
    
    // UI State
    input_value: String,
    error_message: Option<String>,
    sender: Option<tokio::sync::mpsc::Sender<IpcMessage>>,
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    AddPressed,
    DeletePressed(String),
    LockToggled(String, bool),
    TimeframeChanged(String, String),
    
    // Config controls
    LanguageChanged(Language),
    IntervalChanged(u64),
    AutoStartToggled(bool),
    
    // Updates
    CheckUpdates,
    PerformUpdate,
    RestartApp,

    // IPC
    IpcConnected(tokio::sync::mpsc::Sender<IpcMessage>),
    IpcServerMessage(IpcMessage),
    IpcClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    English,
    German,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::English => write!(f, "English"),
            Language::German => write!(f, "Deutsch"),
        }
    }
}

impl Into<language::Language> for Language {
    fn into(self) -> language::Language {
        match self {
            Language::English => language::Language::En,
            Language::German => language::Language::De,
        }
    }
}

impl Application for SettingsApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            SettingsApp {
                charts: Vec::new(),
                config: None,
                update_status: None, // Initial state unknown or idle
                input_value: String::new(),
                error_message: None,
                sender: None,
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("Desktop Widget Settings")
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::InputChanged(val) => {
                self.input_value = val;
                self.error_message = None; // Clear error on type
                Command::none()
            }
            Message::AddPressed => {
                if !self.input_value.is_empty() {
                     let upper_symbol = self.input_value.to_uppercase();
                     self.send_ipc(IpcMessage::AddChart(upper_symbol));
                     self.input_value.clear();
                }
                Command::none()
            }
            Message::DeletePressed(id) => {
                self.send_ipc(IpcMessage::DeleteChart(id));
                Command::none()
            }
            Message::LockToggled(id, locked) => {
                self.send_ipc(IpcMessage::ToggleChartLock(id, locked));
                Command::none()
            }
            Message::TimeframeChanged(id, tf) => {
                self.send_ipc(IpcMessage::SetChartTimeframe(id, tf));
                Command::none()
            }
            Message::LanguageChanged(lang) => {
                let code = match lang {
                    Language::English => "en",
                    Language::German => "de",
                };
                if let Some(cfg) = &mut self.config {
                    cfg.language = code.to_string();
                }
                self.send_ipc(IpcMessage::SetLanguage(code.to_string()));
                Command::none()
            }
            Message::IntervalChanged(min) => {
                if let Some(cfg) = &mut self.config {
                    cfg.update_interval = min;
                }
                self.send_ipc(IpcMessage::SetUpdateInterval(min));
                Command::none()
            }
            Message::AutoStartToggled(enabled) => {
                if let Some(cfg) = &mut self.config {
                    cfg.auto_start = enabled;
                }
                self.send_ipc(IpcMessage::SetAutoStart(enabled));
                Command::none()
            }
            Message::CheckUpdates => {
                // Set explicit checking status immediately for UI feedback
                self.update_status = Some(crate::common::UpdateStatus::Checking("".to_string()));
                self.send_ipc(IpcMessage::CheckForUpdates);
                Command::none()
            }
            Message::PerformUpdate => {
                // Set immediate local feedback
                self.update_status = Some(crate::common::UpdateStatus::Updating);
                self.send_ipc(IpcMessage::PerformUpdate);
                Command::none()
            }
            Message::RestartApp => {
                self.send_ipc(IpcMessage::Restart);
                // Also close this window/process
                // Since this is the main window of the settings process, closing it exits the process.
                iced::window::close(iced::window::Id::MAIN)
            }
            Message::IpcConnected(tx) => {
                self.sender = Some(tx.clone());
                // Request initial state
                Command::perform(async move {
                    let _ = tx.send(IpcMessage::GetCharts).await;
                    let _ = tx.send(IpcMessage::GetConfig).await;
                }, |_| Message::InputChanged("".to_string()))
            }
            Message::IpcServerMessage(msg) => {
                match msg {
                    IpcMessage::Charts(charts) => self.charts = charts,
                    IpcMessage::Config(cfg) => self.config = Some(cfg),
                    IpcMessage::UpdateStatus(status) => self.update_status = Some(status),
                    IpcMessage::Error(err) => self.error_message = Some(err),
                    IpcMessage::Shutdown => {
                        return iced::window::close(iced::window::Id::MAIN);
                    },
                    _ => {}
                }
                Command::none()
            }
            Message::IpcClosed => {
                self.sender = None;
                iced::window::close(iced::window::Id::MAIN)
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let current_lang = if let Some(config) = &self.config {
            if config.language == "de" { Language::German } else { Language::English }
        } else {
            Language::English
        };
        let lang_enum: language::Language = current_lang.into();

        let title_str = language::get_text(lang_enum, TextId::SettingsTitle);
        let title = text(title_str).size(24);

        // General Section
        let general_section = if let Some(config) = &self.config {
            // Re-derive for consistency, though we calculated above. 
            // Actually, let's just use what we have.
            
            let lang_pick = pick_list(
                &[Language::English, Language::German][..],
                Some(current_lang),
                Message::LanguageChanged
            );

            let interval_pick = pick_list(
                &[15u64, 30, 60, 120, 240][..],
                Some(config.update_interval),
                Message::IntervalChanged
            );

            let auto_start = checkbox(language::get_text(lang_enum, TextId::AutoStartup), config.auto_start)
                .on_toggle(Message::AutoStartToggled);

            column![
                text(language::get_text(lang_enum, TextId::General)).size(18),
                row![text(language::get_text(lang_enum, TextId::Language)), lang_pick].spacing(10).align_items(Alignment::Center),
                row![text(language::get_text(lang_enum, TextId::UpdateInterval)), interval_pick].spacing(10).align_items(Alignment::Center),
                auto_start
            ].spacing(10)
        } else {
            column![text("Loading config...")]
        };

        // Charts Section
        let input = text_input(language::get_text(lang_enum, TextId::SymbolPlaceholder), &self.input_value)
            .on_input(Message::InputChanged)
            .on_submit(Message::AddPressed)
            .padding(10)
            .width(Length::Fill);

        let add_btn = button(language::get_text(lang_enum, TextId::AddButton))
            .on_press(Message::AddPressed)
            .padding(10);

        let controls = row![input, add_btn].spacing(10);
        
        let mut controls_column = column![controls];
        if let Some(err) = &self.error_message {
            controls_column = controls_column.push(text(err).style(iced::theme::Text::Color(iced::Color::from_rgb8(255, 100, 100))));
        }

        let mut chart_list = column![].spacing(10);
        for chart in &self.charts {
            // Timeframe Picker
            let timeframe_list = &["1D", "1W", "1M", "3M", "YTD", "1Y"][..];
            // Ensure current timeframe is valid or default (simple check)
            let tf_selected_str = if timeframe_list.contains(&chart.timeframe.as_str()) {
                Some(chart.timeframe.as_str())
            } else {
                Some("1M")
            };
            
            let tf_pick = pick_list(
                timeframe_list,
                tf_selected_str,
                move |tf| Message::TimeframeChanged(chart.id.clone(), tf.to_string())
            ).width(Length::Fixed(80.0));

            // Lock Toggle
            let lock_icon = if chart.locked { crate::icons::lock_icon() } else { crate::icons::unlock_icon() };
            let lock_text_id = if chart.locked { TextId::Locked } else { TextId::Unlocked };
            let lock_btn = tooltip(
                button(svg(lock_icon).width(Length::Fixed(20.0)).height(Length::Fixed(20.0)))
                    .on_press(Message::LockToggled(chart.id.clone(), !chart.locked))
                    .padding(5),
                language::get_text(lang_enum, lock_text_id),
                tooltip::Position::Top
            );

            let del_btn = tooltip(
                button(svg(crate::icons::trash_icon()).width(Length::Fixed(20.0)).height(Length::Fixed(20.0)))
                    .on_press(Message::DeletePressed(chart.id.clone()))
                    .style(iced::theme::Button::Destructive)
                    .padding(5),
                language::get_text(lang_enum, TextId::DeleteButton),
                tooltip::Position::Top
            );

            let row = row![
                text(&chart.symbol).width(Length::Fill).size(18),
                tf_pick,
                lock_btn,
                del_btn
            ]
            .spacing(15)
            .align_items(Alignment::Center)
            .padding(10);
            
            // wrap in container for styling if needed, or just push row
            chart_list = chart_list.push(container(row).style(iced::theme::Container::Box));
        }

        let charts_section = column![
            text(language::get_text(lang_enum, TextId::Charts)).size(18),
            controls_column.spacing(5),
            scrollable(chart_list).height(Length::Fill)
        ].spacing(10).height(Length::Fill);

        // Updates Section
        let (status_text, show_update, show_restart) = match &self.update_status {
            Some(crate::common::UpdateStatus::Checking(_)) => (language::get_text(lang_enum, TextId::UpdateChecking).to_string(), false, false),
            Some(crate::common::UpdateStatus::Available(v)) => (language::get_text(lang_enum, TextId::UpdateAvailableWithVersions).replacen("{}", env!("CARGO_PKG_VERSION"), 1).replacen("{}", v, 1), true, false),
            Some(crate::common::UpdateStatus::UpToDate(_)) => (format!("{} (v{})", language::get_text(lang_enum, TextId::UpdateUpToDate), env!("CARGO_PKG_VERSION")), false, false),
            Some(crate::common::UpdateStatus::Error(e)) => (format!("{} {}", language::get_text(lang_enum, TextId::ErrorPrefix), e), false, false),
            Some(crate::common::UpdateStatus::Updating) => (language::get_text(lang_enum, TextId::UpdateUpdating).to_string(), false, false),
            Some(crate::common::UpdateStatus::Updated(v)) => (format!("v{} {} {}", v, language::get_text(lang_enum, TextId::UpdateSuccess), language::get_text(lang_enum, TextId::UpdateRestart)), false, true),
            None => (format!("v{}", env!("CARGO_PKG_VERSION")), false, false),
        };

        // Determine if "Check for Updates" should be visible
        // Hide if updating, updated, or available (assuming one action at a time)
        let show_check = !show_update && !show_restart && 
                         !matches!(self.update_status, Some(crate::common::UpdateStatus::Updating));

        let mut update_row = row![
            text(status_text).width(Length::Fill),
        ].spacing(10).align_items(Alignment::Center);

        if show_check {
             update_row = update_row.push(button(language::get_text(lang_enum, TextId::UpdateCheck)).on_press(Message::CheckUpdates));
        }

        if show_update {
             update_row = update_row.push(
                 button(language::get_text(lang_enum, TextId::UpdateBtnNow)).on_press(Message::PerformUpdate)
             );
        }
        
        if show_restart {
             update_row = update_row.push(
                 button(language::get_text(lang_enum, TextId::UpdateRestart)).on_press(Message::RestartApp)
             );
        }

        let update_section = column![
            horizontal_rule(1),
            text(language::get_text(lang_enum, TextId::UpdateCheck)).size(18), 
            update_row
        ].spacing(10);

        container(column![
            title,
            vertical_space().height(10.0),
            general_section,
            vertical_space().height(20.0),
            charts_section,
            vertical_space().height(10.0),
            update_section
        ].padding(20).spacing(10))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        struct IpcSub;
        iced::subscription::channel(
            std::any::TypeId::of::<IpcSub>(),
            100,
            |output| async move {
                subscription_logic(output).await;
                std::future::pending().await
            }
        )
    }
}

impl SettingsApp {
    fn send_ipc(&self, msg: IpcMessage) {
        if let Some(tx) = &self.sender {
            let tx = tx.clone();
            let _ = tokio::spawn(async move {
                let _ = tx.send(msg).await;
            });
        }
    }
}

// Helper logic
async fn subscription_logic(mut output: iced::futures::channel::mpsc::Sender<Message>) {
    loop {
        // Connect
        let client = loop {
            match ClientOptions::new().open(PIPE_NAME) {
                Ok(c) => break c,
                Err(_) => {
                     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        };
        
        let (mut reader, mut writer) = tokio::io::split(client);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<IpcMessage>(32);
        
        let _ = output.try_send(Message::IpcConnected(tx));
        
        loop {
            // Helper future for reading
            let read_fut = async {
                let mut len_buf = [0u8; 4];
                if reader.read_exact(&mut len_buf).await.is_err() { return None; }
                let len = u32::from_le_bytes(len_buf) as usize;
                let mut buf = vec![0u8; len];
                if reader.read_exact(&mut buf).await.is_err() { return None; }
                serde_json::from_slice::<IpcMessage>(&buf).ok()
            };

            tokio::select! {
                msg_out = rx.recv() => {
                    match msg_out {
                         Some(msg) => {
                             if let Ok(json) = serde_json::to_string(&msg) {
                                 let bytes = json.as_bytes();
                                 let len = bytes.len() as u32;
                                 if writer.write_all(&len.to_le_bytes()).await.is_err() { break; }
                                 if writer.write_all(bytes).await.is_err() { break; }
                             }
                         }
                         None => break,
                    }
                }
                msg_in = read_fut => {
                    match msg_in {
                        Some(msg) => {
                            let _ = output.try_send(Message::IpcServerMessage(msg));
                        }
                        None => break, // Read error
                    }
                }
            }
        }
        
        let _ = output.try_send(Message::IpcClosed);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
