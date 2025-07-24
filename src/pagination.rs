/// A structure to handle paginated data.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Paginated<T> {
    /// The items on the current page.
    pub items: Vec<T>,
    /// The current page number (1-indexed).
    pub page: usize,
    /// The number of items per page.
    pub page_size: usize,
    /// The total number of items across all pages.
    pub total_items: usize,
    /// The total number of pages.
    pub total_pages: usize,
}

const DEFAULT_PAGE_SIZE: usize = 10;

impl<T> Paginated<T> {
    /// Creates a new `Paginated` instance.
    pub fn new(items: Vec<T>, page: usize) -> Self {
        let total_items = items.len();

        let total_pages = if total_items == 0 {
            1 // Conventionally, an empty list is considered 1 page.
        } else {
            // Ceiling division
            total_items.div_ceil(DEFAULT_PAGE_SIZE)
        };

        // Clamp current_page to be within [1, total_pages]
        let validated_page = page.max(1).min(total_pages);

        Paginated {
            items,
            page: validated_page,
            page_size: DEFAULT_PAGE_SIZE,
            total_items,
            total_pages,
        }
    }

    /// Returns `true` if there is a next page.
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages
    }

    /// Returns `true` if there is a previous page.
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }

    /// Returns a slice of the items on the current page.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty_items() {
        let items: Vec<i32> = vec![];
        let paginated = Paginated::new(items, 1);
        assert_eq!(paginated.page, 1);
        assert_eq!(paginated.total_items, 0);
        assert_eq!(paginated.total_pages, 1, "Empty list should have 1 total page");
        assert_eq!(paginated.get_page_items(), &[] as &[i32]);
        assert!(!paginated.has_prev());
        assert!(!paginated.has_next());
    }

    #[test]
    fn test_new_single_full_page() {
        let items = vec![1, 2, 3, 4, 5];
        let paginated = Paginated::new(items.clone(), 1);
        assert_eq!(paginated.page, 1);
        assert_eq!(paginated.total_items, 5);
        assert_eq!(paginated.total_pages, 1);
        assert_eq!(paginated.get_page_items(), &[1, 2, 3, 4, 5]);
        assert!(!paginated.has_prev());
        assert!(!paginated.has_next());
    }

    #[test]
    fn test_new_multiple_pages_exact_fit() {
        let items = (1..=20).collect::<Vec<i32>>(); // 20 items
        let paginated = Paginated::new(items.clone(), 1);
        assert_eq!(paginated.total_items, 20);
        assert_eq!(paginated.total_pages, 2);
        assert_eq!(paginated.page, 1);

        let paginated_p2 = Paginated::new(items.clone(), 2);
        assert_eq!(paginated_p2.page, 2);
    }

    #[test]
    fn test_new_multiple_pages_with_leftover() {
        let items = (1..=22).collect::<Vec<i32>>(); // 22 items
        let paginated = Paginated::new(items.clone(), 1);
        assert_eq!(paginated.total_items, 22);
        assert_eq!(paginated.total_pages, 3);
    }

    #[test]
    fn test_new_page_clamping() {
        let items = (1..=22).collect::<Vec<i32>>(); // 3 pages
        let paginated_low = Paginated::new(items.clone(), 0); // page 0
        assert_eq!(paginated_low.page, 1, "Page should be clamped to 1 if < 1");

        let paginated_high = Paginated::new(items.clone(), 5); // page 5 (out of 3)
        assert_eq!(paginated_high.page, 3, "Page should be clamped to total_pages");

        let paginated_exact_high = Paginated::new(items.clone(), 3);
        assert_eq!(paginated_exact_high.page, 3);
    }

    #[test]
    fn test_has_prev_next_logic() {
        let items = (1..=22).collect::<Vec<i32>>(); // 3 pages 

        // Page 1
        let p1 = Paginated::new(items.clone(), 1);
        assert!(!p1.has_prev());
        assert!(p1.has_next());

        // Page 2 (middle)
        let p2 = Paginated::new(items.clone(), 2);
        assert!(p2.has_prev());
        assert!(p2.has_next());

        // Page 3 (last)
        let p3 = Paginated::new(items.clone(), 3);
        assert!(p3.has_prev());
        assert!(!p3.has_next());
    }
}
