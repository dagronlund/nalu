pub enum BrowserFilterSection {
    Wildcard,
    WildcardDouble,
    WildcardBefore(String),
    WildcardAfter(String),
    WildcardBoth(String),
    Match(String),
}

pub fn construct_filter(filter: String) -> Vec<BrowserFilterSection> {
    let filter_cleaned = filter.replace('/', ".");
    let filter_sections = filter_cleaned.split('.');
    let mut filters = Vec::new();
    for filter_section in filter_sections {
        match filter_section.len() {
            0 => filters.push(BrowserFilterSection::Match(String::new())),
            1 => match filter_section {
                "*" => filters.push(BrowserFilterSection::Wildcard),
                _ => filters.push(BrowserFilterSection::Match(String::from(filter_section))),
            },
            2 => match (&filter_section[0..1], &filter_section[1..2]) {
                ("*", "*") => filters.push(BrowserFilterSection::WildcardDouble),
                ("*", c) => filters.push(BrowserFilterSection::WildcardBefore(String::from(c))),
                (c, "*") => filters.push(BrowserFilterSection::WildcardAfter(String::from(c))),
                (_, _) => filters.push(BrowserFilterSection::Match(String::from(filter_section))),
            },
            len => match (&filter_section[0..1], &filter_section[len - 1..len]) {
                ("*", "*") => filters.push(BrowserFilterSection::WildcardBoth(String::from(
                    &filter_section[1..len - 1],
                ))),
                ("*", _) => filters.push(BrowserFilterSection::WildcardBefore(String::from(
                    &filter_section[1..len],
                ))),
                (_, "*") => filters.push(BrowserFilterSection::WildcardAfter(String::from(
                    &filter_section[0..len - 1],
                ))),
                (_, _) => filters.push(BrowserFilterSection::Match(String::from(filter_section))),
            },
        }
    }
    filters
}
