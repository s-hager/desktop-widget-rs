use std::error::Error;
use self_update::cargo_crate_version;

pub fn check_update() -> Result<Option<self_update::update::Release>, Box<dyn Error>> {
    let mut status_builder = self_update::backends::github::Update::configure();
    
    status_builder
        .repo_owner("s-hager")
        .repo_name("desktop-widget-rs")
        .bin_name("desktop-widget-rs")
        .show_download_progress(true)
        .current_version(cargo_crate_version!());

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        status_builder.auth_token(&token);
    }
        
    let status = status_builder.build()?
        .get_latest_release()?;

    if self_update::version::bump_is_greater(cargo_crate_version!(), &status.version)? {
        Ok(Some(status))
    } else {
        Ok(None)
    }
}

pub fn perform_update() -> Result<String, Box<dyn Error>> {
    let mut status_builder = self_update::backends::github::Update::configure();
    
    status_builder
        .repo_owner("s-hager")
        .repo_name("desktop-widget-rs")
        .bin_name("desktop-widget-rs")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .no_confirm(true);

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        status_builder.auth_token(&token);
    }
        
    let status = status_builder.build()?
        .update()?;

    println!("Update status: `{}`!", status.version());
    Ok(status.version().to_string())
}

use winit::event_loop::EventLoopProxy;
use crate::common::UserEvent;
use windows::{

    UI::Notifications::{ToastNotification, ToastNotificationManager, ToastTemplateType},
    core::HSTRING,
    Foundation::TypedEventHandler,
};

use crate::language::{Language, TextId, get_text};

pub fn show_update_notification(version: &str, aum_id: &str, proxy: EventLoopProxy<UserEvent>, lang: Language) -> Result<(), Box<dyn Error>> {
    // Create Toast XML
    let toast_xml = ToastNotificationManager::GetTemplateContent(ToastTemplateType::ToastText02)?;
    
    let text_nodes = toast_xml.GetElementsByTagName(&HSTRING::from("text"))?;
    text_nodes.Item(0)?.AppendChild(&toast_xml.CreateTextNode(&HSTRING::from(get_text(lang, TextId::UpdateAvailable)))?)?;
    let body_text = get_text(lang, TextId::UpdateBody).replace("{}", version);
    text_nodes.Item(1)?.AppendChild(&toast_xml.CreateTextNode(&HSTRING::from(body_text))?)?;

    // Create Toast
    let toast = ToastNotification::CreateToastNotification(&toast_xml)?;

    // Handle Click
    let proxy_c = proxy.clone();
    toast.Activated(&TypedEventHandler::new(move |_, _| {
        let _ = proxy_c.send_event(UserEvent::OpenSettings);
        Ok(())
    }))?;

    // Show Toast
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(aum_id))?;
    notifier.Show(&toast)?;

    Ok(())
}
