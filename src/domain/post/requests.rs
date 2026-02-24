use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct PostQuery {
    pub title: Option<QueryTitle>,
    pub created_by_id: Option<CreatedBy>,
    pub filters: Filters,
}

impl TryFrom<GetAllPostsQuery> for PostQuery {
    type Error = String;

    fn try_from(query: GetAllPostsQuery) -> Result<Self, Self::Error> {
        Ok(PostQuery {
            title: (!query.title.is_empty())
                .then(|| QueryTitle::parse(query.title))
                .transpose()?,
            created_by_id: (!query.id.is_empty())
                .then(|| CreatedBy::parse(query.id))
                .transpose()?,
            filters: Filters {
                page: Page::parse(query.page)?,
                limit: Limit::parse(query.limit)?,
                sort: Sort::parse(&query.sort)?,
            },
        })
    }
}

#[derive(Debug)]
pub struct QueryTitle(String);

impl QueryTitle {
    pub fn parse(s: String) -> Result<Self, String> {
        let trimmed = s.trim();

        if trimmed.len() > 100 {
            return Err("Invalid title: cannot exceed 100 characters.".to_string());
        }

        Ok(Self(trimmed.to_string()))
    }
}

impl AsRef<str> for QueryTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct CreatedBy(Uuid);

impl CreatedBy {
    pub fn parse(s: String) -> Result<Self, String> {
        let created_by = Uuid::parse_str(&s).map_err(|_| "Invalid UUID format: created_by")?;
        Ok(Self(created_by))
    }
}

impl AsRef<Uuid> for CreatedBy {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

#[derive(Debug)]
pub struct Page(i32);

impl Page {
    pub fn parse(value: i32) -> Result<Self, String> {
        if value <= 0 {
            return Err("page must be greater than zero".to_string());
        }

        if value > 1_000_000 {
            return Err("page must be a maximum of 1 million".to_string());
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

#[derive(Debug)]
pub struct Limit(i32);

impl Limit {
    pub fn parse(value: i32) -> Result<Self, String> {
        if value <= 0 {
            return Err("limit must be greater than zero".to_string());
        }

        if value > 100 {
            return Err("limit must be a maximum of 100".to_string());
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

#[derive(Debug)]
pub enum SortField {
    Title,
    LikesCount,
    CreatedAt,
}

#[derive(Debug)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug)]
pub struct Sort {
    field: SortField,
    // make this field public, but only within the current crate
    pub(crate) direction: SortDirection,
}

impl Sort {
    pub fn parse(s: &str) -> Result<Self, String> {
        let valid_sorts = [
            "id",
            "title",
            "readtime",
            "likescount",
            "created_at",
            "-id",
            "-title",
            "-readtime",
            "-likescount",
            "-created_at",
        ];

        if !valid_sorts.contains(&s) {
            return Err("invalid sort value".to_string());
        }

        let (field_str, direction) = if let Some(stripped) = s.strip_prefix('-') {
            (stripped, SortDirection::Desc)
        } else {
            (s, SortDirection::Asc)
        };

        let field = match field_str {
            "title" => SortField::Title,
            "created_at" => SortField::CreatedAt,
            "likescount" => SortField::LikesCount,
            _ => return Err("invalid sort value".to_string()),
        };

        Ok(Self { field, direction })
    }

    pub fn to_sql(&self) -> String {
        let column = match self.field {
            SortField::Title => "title",
            SortField::CreatedAt => "created_at",
            SortField::LikesCount => "ARRAY_LENGTH(liked_by, 1)",
        };

        let direction = match (&self.field, &self.direction) {
            (SortField::LikesCount, SortDirection::Desc) => "DESC NULLS LAST",
            (_, SortDirection::Desc) => "DESC",
            (_, SortDirection::Asc) => "ASC",
        };

        format!("{column} {direction}")
    }
}

#[derive(Debug)]
pub struct Filters {
    pub page: Page,
    pub limit: Limit,
    pub sort: Sort,
}

impl Filters {
    pub(crate) fn offset(&self) -> i32 {
        (self.page.value() - 1) * self.limit.value()
    }
}

#[derive(Deserialize, Debug)]
pub struct GetAllPostsQuery {
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub id: String,
}

fn default_sort() -> String {
    "-created_at".to_string()
}

fn default_page() -> i32 {
    1
}

fn default_limit() -> i32 {
    6
}

#[derive(Serialize, Debug)]
pub struct PostData {
    pub id: Uuid,
    pub title: String,
    pub text: String,
    pub img: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub created_by_name: String,
    #[serde(default)]
    pub liked_by: Vec<Uuid>,
}

#[derive(Serialize, Debug)]
pub struct Metadata {
    pub current_page: i32,
    pub page_size: i32,
    pub first_page: i32,
    pub last_page: i32,
    pub total_records: i64,
}

impl Metadata {
    pub(crate) fn calculate(total_records: i64, page: i32, page_size: i32) -> Self {
        let last_page = if total_records == 0 {
            1
        } else {
            (total_records as f64 / page_size as f64).ceil() as i32
        };

        Self {
            current_page: page,
            page_size,
            first_page: 1,
            last_page,
            total_records,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    // `QueryTitle` tests
    #[test]
    fn empty_query_title_is_accepted() {
        let result = QueryTitle::parse("".into());
        assert_ok!(result);
    }

    #[test]
    fn query_title_within_limit_is_accepted() {
        let result = QueryTitle::parse("Valid query".into());
        assert_ok!(result);
    }

    #[test]
    fn query_title_at_max_length_is_accepted() {
        let title = "a".repeat(100);
        let result = QueryTitle::parse(title);
        assert_ok!(result);
    }

    #[test]
    fn query_title_exceeding_limit_is_rejected() {
        let long_title = "a".repeat(101);
        let result = QueryTitle::parse(long_title);
        assert_err!(result);
    }

    #[test]
    fn query_title_with_whitespace_is_trimmed() {
        let title = QueryTitle::parse("  query  ".into()).unwrap();
        assert_eq!(title.as_ref(), "query");
    }

    // `CreatedBy` tests
    #[test]
    fn valid_uuid_is_accepted() {
        let uuid = Uuid::new_v4().to_string();
        let result = CreatedBy::parse(uuid);
        assert_ok!(result);
    }

    #[test]
    fn invalid_uuid_is_rejected() {
        let result = CreatedBy::parse("not-a-uuid".into());
        assert_err!(result);
    }

    #[test]
    fn empty_uuid_string_is_rejected() {
        let result = CreatedBy::parse("".into());
        assert_err!(result);
    }

    #[test]
    fn malformed_uuid_is_rejected() {
        let result = CreatedBy::parse("123e4567-e89b-12d3-a456".into());
        assert_err!(result);
    }

    // `Page` tests
    #[test]
    fn page_zero_is_rejected() {
        let result = Page::parse(0);
        assert_err!(result);
    }

    #[test]
    fn page_negative_is_rejected() {
        let result = Page::parse(-1);
        assert_err!(result);
    }

    #[test]
    fn page_one_is_accepted() {
        let result = Page::parse(1);
        assert_ok!(result);
    }

    #[test]
    fn page_valid_is_accepted() {
        let result = Page::parse(100);
        assert_ok!(result);
    }

    #[test]
    fn page_at_max_is_accepted() {
        let result = Page::parse(1_000_000);
        assert_ok!(result);
    }

    #[test]
    fn page_exceeding_max_is_rejected() {
        let result = Page::parse(1_000_001);
        assert_err!(result);
    }

    #[test]
    fn page_value_returns_correct_number() {
        let page = Page::parse(42).unwrap();
        assert_eq!(page.value(), 42);
    }

    // `Limit` tests
    #[test]
    fn limit_zero_is_rejected() {
        let result = Limit::parse(0);
        assert_err!(result);
    }

    #[test]
    fn limit_negative_is_rejected() {
        let result = Limit::parse(-1);
        assert_err!(result);
    }

    #[test]
    fn limit_one_is_accepted() {
        let result = Limit::parse(1);
        assert_ok!(result);
    }

    #[test]
    fn limit_valid_is_accepted() {
        let result = Limit::parse(10);
        assert_ok!(result);
    }

    #[test]
    fn limit_at_max_is_accepted() {
        let result = Limit::parse(100);
        assert_ok!(result);
    }

    #[test]
    fn limit_exceeding_max_is_rejected() {
        let result = Limit::parse(101);
        assert_err!(result);
    }

    #[test]
    fn limit_value_returns_correct_number() {
        let limit = Limit::parse(25).unwrap();
        assert_eq!(limit.value(), 25);
    }

    // `Sort` tests
    #[test]
    fn valid_sort_title_is_accepted() {
        let result = Sort::parse("title");
        assert_ok!(result);
    }

    #[test]
    fn valid_sort_created_at_is_accepted() {
        let result = Sort::parse("created_at");
        assert_ok!(result);
    }

    #[test]
    fn valid_sort_likescount_is_accepted() {
        let result = Sort::parse("likescount");
        assert_ok!(result);
    }

    #[test]
    fn valid_desc_sort_title_is_accepted() {
        let result = Sort::parse("-title");
        assert_ok!(result);
    }

    #[test]
    fn valid_desc_sort_created_at_is_accepted() {
        let result = Sort::parse("-created_at");
        assert_ok!(result);
    }

    #[test]
    fn valid_desc_sort_likescount_is_accepted() {
        let result = Sort::parse("-likescount");
        assert_ok!(result);
    }

    #[test]
    fn invalid_sort_field_is_rejected() {
        let result = Sort::parse("invalid_field");
        assert_err!(result);
    }

    #[test]
    fn sort_with_multiple_dashes_is_rejected() {
        let result = Sort::parse("--title");
        assert_err!(result);
    }

    #[test]
    fn empty_sort_string_is_rejected() {
        let result = Sort::parse("");
        assert_err!(result);
    }

    #[test]
    fn sort_to_sql_title_asc() {
        let sort = Sort::parse("title").unwrap();
        assert_eq!(sort.to_sql(), "title ASC");
    }

    #[test]
    fn sort_to_sql_title_desc() {
        let sort = Sort::parse("-title").unwrap();
        assert_eq!(sort.to_sql(), "title DESC");
    }

    #[test]
    fn sort_to_sql_created_at_asc() {
        let sort = Sort::parse("created_at").unwrap();
        assert_eq!(sort.to_sql(), "created_at ASC");
    }

    #[test]
    fn sort_to_sql_created_at_desc() {
        let sort = Sort::parse("-created_at").unwrap();
        assert_eq!(sort.to_sql(), "created_at DESC");
    }

    #[test]
    fn sort_to_sql_likescount_asc() {
        let sort = Sort::parse("likescount").unwrap();
        assert_eq!(sort.to_sql(), "ARRAY_LENGTH(liked_by, 1) ASC");
    }

    #[test]
    fn sort_to_sql_likescount_desc() {
        let sort = Sort::parse("-likescount").unwrap();
        assert_eq!(sort.to_sql(), "ARRAY_LENGTH(liked_by, 1) DESC NULLS LAST");
    }

    // `Filters` tests
    #[test]
    fn filters_offset_calculation_first_page() {
        let filters = Filters {
            page: Page::parse(1).unwrap(),
            limit: Limit::parse(10).unwrap(),
            sort: Sort::parse("created_at").unwrap(),
        };
        assert_eq!(filters.offset(), 0);
    }

    #[test]
    fn filters_offset_calculation_second_page() {
        let filters = Filters {
            page: Page::parse(2).unwrap(),
            limit: Limit::parse(10).unwrap(),
            sort: Sort::parse("created_at").unwrap(),
        };
        assert_eq!(filters.offset(), 10);
    }

    #[test]
    fn filters_offset_calculation_with_different_limit() {
        let filters = Filters {
            page: Page::parse(3).unwrap(),
            limit: Limit::parse(25).unwrap(),
            sort: Sort::parse("created_at").unwrap(),
        };
        assert_eq!(filters.offset(), 50);
    }

    // `Metadata` tests
    #[test]
    fn metadata_calculates_last_page_correctly() {
        let metadata = Metadata::calculate(100, 1, 10);
        assert_eq!(metadata.current_page, 1);
        assert_eq!(metadata.page_size, 10);
        assert_eq!(metadata.first_page, 1);
        assert_eq!(metadata.last_page, 10);
        assert_eq!(metadata.total_records, 100);
    }

    #[test]
    fn metadata_handles_zero_records() {
        let metadata = Metadata::calculate(0, 1, 10);
        assert_eq!(metadata.last_page, 1);
        assert_eq!(metadata.total_records, 0);
    }

    #[test]
    fn metadata_rounds_up_partial_pages() {
        let metadata = Metadata::calculate(95, 1, 10);
        assert_eq!(metadata.last_page, 10);
    }

    #[test]
    fn metadata_handles_exact_page_boundary() {
        let metadata = Metadata::calculate(100, 1, 10);
        assert_eq!(metadata.last_page, 10);
    }

    #[test]
    fn metadata_handles_single_record() {
        let metadata = Metadata::calculate(1, 1, 10);
        assert_eq!(metadata.last_page, 1);
    }

    #[test]
    fn metadata_with_large_page_size() {
        let metadata = Metadata::calculate(50, 1, 100);
        assert_eq!(metadata.last_page, 1);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn query_title_under_limit_is_accepted(
            title in r"[a-zA-Z0-9 ]{0,100}",
        ) {
            let result = QueryTitle::parse(title);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn page_in_valid_range_is_accepted(
            page in 1..=1_000_000i32,
        ) {
            let result = Page::parse(page);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn limit_in_valid_range_is_accepted(
            limit in 1..=100i32,
        ) {
            let result = Limit::parse(limit);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn offset_calculation_is_correct(
            page in 1..=1000i32,
            limit in 1..=100i32,
        ) {
            let filters = Filters {
                page: Page::parse(page).unwrap(),
                limit: Limit::parse(limit).unwrap(),
                sort: Sort::parse("created_at").unwrap(),
            };
            let expected_offset = (page - 1) * limit;
            prop_assert_eq!(filters.offset(), expected_offset);
        }
    }
}
