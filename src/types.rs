use std::borrow::Cow;

use chrono::{Date, DateTime, TimeZone};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug)]
pub enum Error {
    JsonError(serde_json::Error),
    RequestError(reqwest::Error),
    UnexpectedResponse(String),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JsonError(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::RequestError(err)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Resolution {
    Country,
    City,
    Dma,
}

impl Serialize for Resolution {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = match self {
            Resolution::Country => "COUNTRY",
            Resolution::City => "CITY",
            Resolution::Dma => "DMA",
        };
        serializer.serialize_str(formatted)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Source {
    Search,
    Images,
    News,
    Videos,
    Shopping,
}

impl Serialize for Source {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = match self {
            Source::Search => "",
            Source::Images => "images",
            Source::News => "news",
            Source::Videos => "youtube",
            Source::Shopping => "froogle",
        };
        serializer.serialize_str(formatted)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum Category {
    All = 0,
    Entertainment = 3,
    Electronics = 5,
    Finance = 7,
    Games = 8,
    Home = 11,
    Business = 12,
    Internet = 13,
    Society = 14,
    News = 16,
    Shopping = 18,
    Law = 19,
    Sports = 20,
    Literature = 22,
    RealEstate = 29,
    Fitness = 44,
    Health = 45,
    Vehicles = 47,
    Hobbies = 65,
    Pets = 66,
    Travel = 67,
    Food = 71,
    Science = 174,
    Communities = 299,
    Reference = 533,
    Education = 958,
}

impl Serialize for Category {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(*self as u32)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SearchType {
    TimeSeries,
    Region,
    RelatedTopics,
    RelatedQueries,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Query<'a> {
    comparison_item: Vec<QueryItem<'a>>,
    category: Category,
    property: Source,
}

impl<'a> Query<'a> {
    pub fn new(items: Vec<QueryItem>) -> Query {
        Query {
            comparison_item: items,
            category: Category::All,
            property: Source::Search,
        }
    }

    pub fn by_keyword(keyword: String, time: Timeframe) -> Self {
        Query::new(vec![QueryItem::by_keyword(keyword, time)])
    }

    pub fn items(&self) -> &[QueryItem] {
        &self.comparison_item
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryItem<'a> {
    keyword: Cow<'a, str>,
    geo: Option<Cow<'a, str>>,
    time: Timeframe,
}

impl<'a> QueryItem<'a> {
    pub fn by_keyword<S: Into<Cow<'a, str>>>(keyword: S, time: Timeframe) -> Self {
        QueryItem {
            keyword: keyword.into(),
            geo: None,
            time,
        }
    }

    pub fn by_keyword_with_geo<S: Into<Cow<'a, str>>>(keyword: S, region: S, time: Timeframe) -> Self {
        QueryItem {
            keyword: keyword.into(),
            geo: Some(region.into()),
            time,
        }
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }
}

#[derive(Debug, Clone)]
pub struct Timeframe {
    start: Date<chrono::offset::Utc>,
    end: Date<chrono::offset::Utc>,
}

impl Timeframe {
    pub fn new(start: Date<chrono::offset::Utc>, end: Date<chrono::offset::Utc>) -> Timeframe {
        Timeframe { start, end }
    }

    pub fn default() -> Timeframe {
        Timeframe {
            start: chrono::Utc.ymd(2014, 1, 1),
            end: chrono::Utc::now().date(),
        }
    }

    pub fn formatted(&self) -> String {
        format!("{} {}", self.start.format("%Y-%m-%d"), self.end.format("%Y-%m-%d"))
    }
}

impl Serialize for Timeframe {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.formatted())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionEntry {
    pub coordinates: Option<Coordinates>,
    pub geo_code: String,
    pub geo_name: String,
    pub value: Vec<u8>,
    pub has_data: Vec<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Coordinates {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeriesEntry {
    #[serde(with = "trends_time_format")]
    pub time: DateTime<chrono::offset::Utc>,
    pub formatted_time: String,
    pub value: Vec<u8>,
    pub has_data: Vec<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegionData {
    #[serde(rename = "geoMapData")]
    pub entries: Vec<RegionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeSeriesData {
    #[serde(rename = "timelineData")]
    pub entries: Vec<TimeSeriesEntry>,
}

mod trends_time_format {
    use serde::de::Error;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<chrono::DateTime<chrono::offset::Utc>, D::Error>
    where
        D::Error: serde::de::Error,
    {
        let str = String::deserialize(deserializer)?;
        let secs: i64 = str.parse().map_err(D::Error::custom)?;
        let ndt = chrono::NaiveDateTime::from_timestamp(secs, 0);
        Ok(chrono::DateTime::from_utc(ndt, chrono::offset::Utc))
    }
}
