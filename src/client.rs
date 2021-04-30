use reqwest::header::HeaderValue;
use reqwest::{Client, Method, Request, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Feature {
    DataRequest(RequestParameters),
    Other { id: String },
}

#[derive(Debug, Clone, Deserialize)]
struct RequestParameters {
    token: String,
    id: String,
    request: serde_json::Value,
}

impl RequestParameters {
    fn resolution(&mut self, resolution: Resolution) -> Result<(), serde_json::Error> {
        self.request["resolution"] = serde_json::to_value(resolution)?;
        Ok(())
    }

    fn source(&mut self, source: Source) -> Result<(), serde_json::Error> {
        self.request["requestOptions"]["property"] = serde_json::to_value(source)?;
        Ok(())
    }

    fn category(&mut self, category: Category) -> Result<(), serde_json::Error> {
        self.request["requestOptions"]["category"] = serde_json::to_value(category)?;
        Ok(())
    }

    fn include_low_volume_geos(&mut self, include: bool) -> Result<(), serde_json::Error> {
        self.request["includeLowSearchVolumeGeos"] = serde_json::to_value(include)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ExploreResponse {
    widgets: Vec<Feature>,
}

impl ExploreResponse {
    fn get_request(&self, search: SearchType) -> Option<&RequestParameters> {
        let id = match search {
            SearchType::TimeSeries => "TIMESERIES",
            SearchType::Region => "GEO_MAP",
            SearchType::RelatedTopics => "RELATED_TOPICS",
            SearchType::RelatedQueries => "RELATED_QUERIES",
        };
        self.widgets.iter().find_map(|item| match item {
            Feature::DataRequest(desc) if id == desc.id => Some(desc),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TimeSeriesResponse {
    default: TimeSeriesData,
}

#[derive(Debug, Clone, Deserialize)]
struct GeoDataResponse {
    default: RegionData,
}

pub struct TrendsClient {
    client: Client,
    locale: String,
}

impl TrendsClient {
    pub fn new(locale: String) -> TrendsClient {
        TrendsClient {
            client: Client::new(),
            locale,
        }
    }

    pub async fn interest_by_time(
        &self,
        query: &Query<'_>,
        source: Source,
        category: Category,
    ) -> Result<TimeSeriesData, Error> {
        let search = SearchType::TimeSeries;
        let mut item = self.explore(query, search).await?;
        item.source(source)?;
        item.category(category)?;

        let resp: TimeSeriesResponse = self.query(&item, search).await?;
        Ok(resp.default)
    }

    pub async fn interest_by_region(
        &self,
        query: &Query<'_>,
        resolution: Resolution,
        source: Source,
        category: Category,
        include_low_volume_regions: bool,
    ) -> Result<RegionData, Error> {
        let search = SearchType::Region;
        let mut item = self.explore(query, search).await?;
        item.resolution(resolution)?;
        item.source(source)?;
        item.category(category)?;
        item.include_low_volume_geos(include_low_volume_regions)?;

        let resp: GeoDataResponse = self.query(&item, search).await?;
        Ok(resp.default)
    }

    async fn query<A: DeserializeOwned>(&self, params: &RequestParameters, search: SearchType) -> Result<A, Error> {
        let req = self
            .client
            .request(Method::GET, Self::endpoint(search))
            .query(&[
                ("hl", self.locale.as_str()),
                ("tz", "0"),
                ("token", &params.token),
                ("req", &serde_json::to_string(&params.request)?),
            ])
            .build()?;

        let body = self.run_with_retry(req).await?.text().await?;
        Ok(serde_json::from_str(&body[5..])?)
    }

    async fn explore(&self, query: &Query<'_>, search: SearchType) -> Result<RequestParameters, Error> {
        let req = self
            .client
            .request(Method::GET, "https://trends.google.com/trends/api/explore")
            .query(&[
                ("hl", self.locale.as_str()),
                ("tz", "0"),
                ("req", &serde_json::to_string(query)?),
            ])
            .build()?;

        let body = self.run_with_retry(req).await?.text().await?;
        let resp: ExploreResponse = serde_json::from_str(&body[4..])?;

        let item = resp
            .get_request(search)
            .ok_or_else(|| Error::UnexpectedResponse("Search feature unavailable".to_owned()))?;
        Ok(item.clone())
    }

    async fn run_with_retry(&self, req: Request) -> Result<Response, Error> {
        let mut req_copy = Request::new(req.method().clone(), req.url().clone());
        *req_copy.headers_mut() = req.headers().clone();

        let resp = self.client.execute(req).await?;
        match resp.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                if let Some(val) = resp
                    .headers()
                    .get("set-cookie")
                    .and_then(|val| val.to_str().ok())
                    .and_then(|str| str.split(';').next())
                {
                    let header = HeaderValue::from_str(val).unwrap();
                    req_copy.headers_mut().insert("cookie", header);
                }
                Ok(self.client.execute(req_copy).await?)
            }
            StatusCode::OK => Ok(resp),
            _ => Err(Error::UnexpectedResponse(resp.text().await?)),
        }
    }

    fn endpoint<'a>(search: SearchType) -> &'a str {
        match search {
            SearchType::TimeSeries => "https://trends.google.com/trends/api/widgetdata/multiline",
            SearchType::Region => "https://trends.google.com/trends/api/widgetdata/comparedgeo",
            SearchType::RelatedTopics => "https://trends.google.com/trends/api/widgetdata/relatedsearches",
            SearchType::RelatedQueries => "https://trends.google.com/trends/api/widgetdata/relatedsearches",
        }
    }
}
