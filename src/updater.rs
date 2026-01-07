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
