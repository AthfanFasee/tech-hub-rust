use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
