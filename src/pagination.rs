/// Pagination structure to handle paginated data (labels, repositories, etc.)
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
}

impl<T> Paginated<T> {
    pub fn new(items: Vec<T>, page: usize) -> Self {
        let page_size = 10; // Default page size
        let total_items = items.len();
        let total_pages = (total_items + 9) / page_size;

        Paginated { items, page, page_size, total_items, total_pages }
    }

    pub fn has_next(&self) -> bool {
        self.page < self.total_pages
    }

    pub fn has_prev(&self) -> bool {
        self.page > 1
    }

    pub fn get_page_items(&self) -> &[T] {
        if self.items.is_empty() || self.page_size == 0 {
            return &[];
        }
        let start = (self.page.saturating_sub(1)) * self.page_size;
        if start >= self.items.len() {
            // Requested page is out of bounds
            return &[];
        }
        let end = (start + self.page_size).min(self.items.len());
        &self.items[start..end]
    }
}
