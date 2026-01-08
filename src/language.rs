use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local, Datelike};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    En,
    De,
}

impl Default for Language {
    fn default() -> Self {
        Language::En
    }
}

impl Language {
    pub fn as_str(&self) -> &str {
        match self {
            Language::En => "EN",
            Language::De => "DE",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Language::En => Language::De,
            Language::De => Language::En,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextId {
    SettingsTitle,
    AddButton,
    DeleteButton,
    AutoStartup,
    UpdateInterval,
    ActiveCharts,
    Quit,
    SettingsMenu,
    ErrorPrefix,
    SymbolNotFound,
    FetchError,
    NoQuotesFound,
    WeekDataError,
    UpdateCheck,
    UpdateChecking,
    UpdateUpToDate,
    UpdateBtnNow,
    UpdateUpdating,
    UpdateRestart,
    UpdateError,
    UpdateAvailable,
    UpdateBody, // Version {} is available.\nClick to open settings.
}

pub fn get_text(lang: Language, id: TextId) -> &'static str {
    match lang {
        Language::En => match id {
            TextId::SettingsTitle => "Settings",
            TextId::AddButton => "Add",
            TextId::DeleteButton => "Del",
            TextId::AutoStartup => "Auto Startup:",
            TextId::UpdateInterval => "Update Interval (min):",
            TextId::ActiveCharts => "Active Charts:",
            TextId::Quit => "Quit",
            TextId::SettingsMenu => "Settings",
            TextId::ErrorPrefix => "Error:",
            TextId::SymbolNotFound => "Symbol not found",
            TextId::FetchError => "Fetch error:",
            TextId::NoQuotesFound => "No quotes found",
            TextId::WeekDataError => "No quotes found for 1W",
            TextId::UpdateCheck => "Check for Updates",
            TextId::UpdateChecking => "Checking...",
            TextId::UpdateUpToDate => "Up to date",
            TextId::UpdateBtnNow => "Update Now",
            TextId::UpdateUpdating => "Updating...",
            TextId::UpdateRestart => "Restart",
            TextId::UpdateError => "Error!",
            TextId::UpdateAvailable => "Update Available",
            TextId::UpdateBody => "Version {} is available.\nClick to open settings.",
        },
        Language::De => match id {
            TextId::SettingsTitle => "Einstellungen",
            TextId::AddButton => "Hinzu",
            TextId::DeleteButton => "Lösch",
            TextId::AutoStartup => "Autostart:",
            TextId::UpdateInterval => "Aktualisierung (Min):",
            TextId::ActiveCharts => "Aktive Charts:",
            TextId::Quit => "Beenden",
            TextId::SettingsMenu => "Einstellungen",
            TextId::ErrorPrefix => "Fehler:",
            TextId::SymbolNotFound => "Symbol nicht gefunden",
            TextId::FetchError => "Abruf-Fehler:",
            TextId::NoQuotesFound => "Keine Kurse gefunden",
            TextId::WeekDataError => "Keine Kurse für 1W gefunden",
            TextId::UpdateCheck => "Updates suchen",
            TextId::UpdateChecking => "Suche...",
            TextId::UpdateUpToDate => "Aktuell",
            TextId::UpdateBtnNow => "Jetzt Aktualisieren",
            TextId::UpdateUpdating => "Aktualisiere...",
            TextId::UpdateRestart => "Neustarten",
            TextId::UpdateError => "Fehler!",
            TextId::UpdateAvailable => "Update verfügbar",
            TextId::UpdateBody => "Version {} ist verfügbar.\nKlicken Sie hier, um die Einstellungen zu öffnen.",
        },
    }
}

pub fn get_month_name(lang: Language, month: u32) -> &'static str {
    match lang {
        Language::En => match month {
            1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr", 5 => "May", 6 => "Jun",
            7 => "Jul", 8 => "Aug", 9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
            _ => "",
        },
        Language::De => match month {
            1 => "Jan", 2 => "Feb", 3 => "Mär", 4 => "Apr", 5 => "Mai", 6 => "Jun",
            7 => "Jul", 8 => "Aug", 9 => "Sep", 10 => "Okt", 11 => "Nov", 12 => "Dez",
            _ => "",
        },
    }
}

pub fn get_day_name(lang: Language, weekday: chrono::Weekday) -> &'static str {
    match lang {
        Language::En => match weekday {
            chrono::Weekday::Mon => "Mon", chrono::Weekday::Tue => "Tue", chrono::Weekday::Wed => "Wed",
            chrono::Weekday::Thu => "Thu", chrono::Weekday::Fri => "Fri", chrono::Weekday::Sat => "Sat",
            chrono::Weekday::Sun => "Sun",
        },
        Language::De => match weekday {
            chrono::Weekday::Mon => "Mo", chrono::Weekday::Tue => "Di", chrono::Weekday::Wed => "Mi",
            chrono::Weekday::Thu => "Do", chrono::Weekday::Fri => "Fr", chrono::Weekday::Sat => "Sa",
            chrono::Weekday::Sun => "So",
        },
    }
}

// Formatting Helpers

pub fn format_time(date: DateTime<Local>) -> String {
    date.format("%H:%M").to_string()
}

pub fn format_weekday_time(lang: Language, date: DateTime<Local>) -> String {
    let day_name = get_day_name(lang, date.weekday());
    format!("{} {}", day_name, date.format("%H:%M"))
}

pub fn format_month_day(lang: Language, date: DateTime<Local>) -> String {
    let month_name = get_month_name(lang, date.month());
    match lang {
        Language::En => format!("{} {}", month_name, date.day()),
        Language::De => format!("{}. {}", date.day(), month_name),
    }
}

#[derive(Debug)]
pub enum AppError {
    FetchError(String),
    NoQuotesFound,
    WeekDataError, // "No quotes found for 1W"
}

pub fn get_error_text(lang: Language, error: &AppError) -> String {
    match lang {
        Language::En => match error {
            AppError::FetchError(_) => "Failed to fetch data from Yahoo Finance".to_string(), // Simplify dynamic error? Or include it?
            AppError::NoQuotesFound => "No quotes found".to_string(),
            AppError::WeekDataError => "No quotes found for 1W".to_string(),
        },
        Language::De => match error {
            AppError::FetchError(_) => "Fehler beim Abrufen der Daten von Yahoo Finance".to_string(),
            AppError::NoQuotesFound => "Keine Kurse gefunden".to_string(),
            AppError::WeekDataError => "Keine Kurse für 1W gefunden".to_string(),
        },
    }
}
