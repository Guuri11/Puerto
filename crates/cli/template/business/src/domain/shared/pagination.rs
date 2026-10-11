/// The page size when the caller does not choose one.
pub const DEFAULT_PAGE_LIMIT: u32 = 20;
/// The largest page anyone can ask for: no list reads an unbounded table.
pub const MAX_PAGE_LIMIT: u32 = 100;

/// Which slice of a list to read. The fields are private, so every request goes through `new`,
/// which applies the cap — no route, use case or adapter can ask for more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    limit: u32,
    offset: u32,
}

impl PageRequest {
    /// `limit` defaults to [`DEFAULT_PAGE_LIMIT`] and is clamped to `1..=MAX_PAGE_LIMIT`;
    /// `offset` defaults to 0.
    pub fn new(limit: Option<u32>, offset: Option<u32>) -> Self {
        Self {
            limit: limit.unwrap_or(DEFAULT_PAGE_LIMIT).clamp(1, MAX_PAGE_LIMIT),
            offset: offset.unwrap_or(0),
        }
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self::new(None, None)
    }
}

/// One page of a list, and how many items the whole list has.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    /// The limit actually applied — after the cap.
    pub limit: u32,
    pub offset: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, request: PageRequest) -> Self {
        Self {
            items,
            total,
            limit: request.limit(),
            offset: request.offset(),
        }
    }

    /// The same page, with every item converted (a DTO, a view model).
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            limit: self.limit,
            offset: self.offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_take_the_defaults_when_nothing_is_asked() {
        let page = PageRequest::new(None, None);
        assert_eq!(page.limit(), DEFAULT_PAGE_LIMIT);
        assert_eq!(page.offset(), 0);
    }

    #[test]
    fn should_cap_the_limit_at_the_maximum() {
        let page = PageRequest::new(Some(MAX_PAGE_LIMIT + 1), Some(40));
        assert_eq!(page.limit(), MAX_PAGE_LIMIT);
        assert_eq!(page.offset(), 40);
    }

    #[test]
    fn should_raise_a_zero_limit_to_one() {
        assert_eq!(PageRequest::new(Some(0), None).limit(), 1);
    }

    #[test]
    fn should_keep_the_applied_limit_and_offset_in_the_page() {
        let page = Page::new(vec!["a", "b"], 7, PageRequest::new(Some(2), Some(4)));
        let mapped = page.map(str::len);
        assert_eq!(
            mapped,
            Page {
                items: vec![1, 1],
                total: 7,
                limit: 2,
                offset: 4
            }
        );
    }
}
