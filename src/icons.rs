use iced::widget::svg;

pub fn lock_icon() -> svg::Handle {
    svg::Handle::from_memory(br#"
<svg viewBox="-5 -2 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
<path d="M12 10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2V5a5 5 0 1 1 10 0v5zm-5 7a2 2 0 1 0 0-4 2 2 0 0 0 0 4zm3-7V5a3 3 0 1 0-6 0v5h6z"/>
</svg>"#.to_vec())
}

pub fn unlock_icon() -> svg::Handle {
    svg::Handle::from_memory(br#"
<svg viewBox="-5 -2 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
<path d="M12 5h-2a3 3 0 1 0-6 0v5h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2V5a5 5 0 1 1 10 0zM7 17a2 2 0 1 0 0-4 2 2 0 0 0 0 4z"/>
</svg>"#.to_vec())
}

pub fn trash_icon() -> svg::Handle {
    svg::Handle::from_memory(br#"
<svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
<path d="M5 20C5 21.103 5.897 22 7 22H17C18.103 22 19 21.103 19 20V8H5V20ZM9 19C9 19.552 8.552 20 8 20C7.448 20 7 19.552 7 19V10C7 9.448 7.448 9 8 9C8.552 9 9 9.448 9 10V19ZM13 19C13 19.552 12.552 20 12 20C11.448 20 11 19.552 11 19V10C11 9.448 11.448 9 12 9C12.552 9 13 9.448 13 10V19ZM17 19C17 19.552 16.552 20 16 20C15.448 20 15 19.552 15 19V10C15 9.448 15.448 9 16 9C16.552 9 17 9.448 17 10V19ZM16 4V3C16 2.448 15.552 2 15 2H9C8.448 2 8 2.448 8 3V4H4V6H20V4H16Z"/>
</svg>"#.to_vec())
}
