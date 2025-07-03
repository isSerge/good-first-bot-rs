use super::TelegramMessagingService;
use crate::pagination::Paginated;

#[test]
fn test_format_paginated_message_text() {
    let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let paginated =
        Paginated { items: items.clone(), page: 1, total_pages: 2, total_items: 20, page_size: 10 };

    let text =
        TelegramMessagingService::format_paginated_message_text("Test Title", &paginated, "items");

    assert_eq!(text, "Test Title (Page 1 of 2)\nTotal items: 20");
}

#[test]
fn test_format_paginated_message_text_no_items() {
    let items: Vec<i32> = vec![];
    let paginated = Paginated { items, page: 1, total_pages: 1, total_items: 0, page_size: 10 };

    let text =
        TelegramMessagingService::format_paginated_message_text("Test Title", &paginated, "items");

    assert_eq!(text, "Test Title\n\nNo items found.");
}
