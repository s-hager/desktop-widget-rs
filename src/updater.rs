use std::error::Error;
use self_update::cargo_crate_version;



pub fn check_update(use_prereleases: bool) -> Result<Option<self_update::update::Release>, Box<dyn Error>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("s-hager")
        .repo_name("desktop-widget-rs")
        .build()?
        .fetch()?;

    for release in releases {
        let is_prerelease = release.name.to_lowercase().contains("beta") 
                         || release.name.to_lowercase().contains("alpha") 
                         || release.name.to_lowercase().contains("rc")
                         || release.name.to_lowercase().contains("dev")
                         || release.version.contains("-");
        
        if use_prereleases || !is_prerelease {
             // Allow update if the versions are different (upgrade or downgrade)
             if release.version != cargo_crate_version!() {
                return Ok(Some(release));
             } else {
                 return Ok(None);
             }
        }
    }
    
    Ok(None)
}

pub fn perform_update(use_prereleases: bool) -> Result<String, Box<dyn Error>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("s-hager")
        .repo_name("desktop-widget-rs")
        .build()?
        .fetch()?;
    
    let target_release = releases.into_iter().find(|release| {
         let is_prerelease = release.name.to_lowercase().contains("beta") 
                          || release.name.to_lowercase().contains("alpha") 
                          || release.name.to_lowercase().contains("rc")
                          || release.name.to_lowercase().contains("dev")
                          || release.version.contains("-");
         use_prereleases || !is_prerelease
    });

    if let Some(release) = target_release {
         // Allow update if the versions are different (upgrade or downgrade)
         if release.version != cargo_crate_version!() {
             
             let mut status_builder = self_update::backends::github::Update::configure();
             status_builder
                .repo_owner("s-hager")
                .repo_name("desktop-widget-rs")
                .bin_name("desktop-widget-rs")
                .show_download_progress(true)
                .current_version(cargo_crate_version!())
                .target_version_tag(&format!("v{}", release.version))
                .no_confirm(true);

            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                status_builder.auth_token(&token);
            }
            
            let status = status_builder.build()?.update()?;
            log::info!("Update status: `{}`!", status.version());
            return Ok(status.version().to_string());
         }
    }
    
    Ok(cargo_crate_version!().to_string())
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
