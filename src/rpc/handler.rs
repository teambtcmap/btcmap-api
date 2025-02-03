use crate::{conf::Conf, Result};
use actix_web::{
    dev::ServiceResponse,
    middleware::ErrorHandlerResponse,
    post,
    web::{Data, Json},
};
use deadpool_sqlite::Pool;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: RpcMethod,
    pub params: Option<Value>,
    pub id: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcMethod {
    // element
    GetElement,
    SetElementTag,
    RemoveElementTag,
    GetBoostedElements,
    BoostElement,
    AddElementComment,
    PaywallAddElementCommentQuote,
    PaywallAddElementComment,
    PaywallGetBoostElementQuote,
    PaywallBoostElement,
    GenerateElementIssues,
    SyncElements,
    GenerateElementIcons,
    GenerateElementCategories,
    // area
    AddArea,
    GetArea,
    SetAreaTag,
    RemoveAreaTag,
    SetAreaIcon,
    RemoveArea,
    GetTrendingCountries,
    GetMostCommentedCountries,
    GetTrendingCommunities,
    GenerateAreasElementsMapping,
}

#[derive(Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<RpcError>,
    pub id: Value,
}

#[derive(Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    fn parse_error(data: Option<Value>) -> Self {
        Self {
            code: -32700,
            message: "Parse error".into(),
            data,
        }
    }
    fn server_error(data: Option<Value>) -> Self {
        Self {
            code: -32000,
            message: "Server error".into(),
            data,
        }
    }
}

impl RpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(error),
            id: Value::Null,
        }
    }

    pub fn from<R>(id: Value, val: R) -> Result<Self>
    where
        R: Serialize,
    {
        Ok(Self::success(id, serde_json::to_value(&val)?))
    }

    fn invalid_request(id: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(RpcError {
                code: -32600,
                message: "Invalid Request".into(),
                data: None,
            }),
            id,
        }
    }
}

#[post("")]
async fn handle(req: Json<Value>, pool: Data<Pool>, conf: Data<Conf>) -> Result<Json<RpcResponse>> {
    let req: RpcRequest = match serde_json::from_value(req.into_inner()) {
        Ok(req) => req,
        Err(e) => {
            let data = Value::String(e.to_string());
            let e = RpcError::parse_error(Some(data));
            return Ok(Json(RpcResponse::error(e)));
        }
    };
    if req.jsonrpc != "2.0" {
        return Ok(Json(RpcResponse::invalid_request(Value::Null)));
    }
    let res: RpcResponse = match req.method {
        RpcMethod::GetElement => RpcResponse::from(
            req.id.clone(),
            super::get_element::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SetElementTag => RpcResponse::from(
            req.id.clone(),
            super::set_element_tag::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::RemoveElementTag => RpcResponse::from(
            req.id.clone(),
            super::remove_element_tag::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GetBoostedElements => RpcResponse::from(
            req.id.clone(),
            super::get_boosted_elements::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::BoostElement => RpcResponse::from(
            req.id.clone(),
            super::boost_element::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::AddElementComment => RpcResponse::from(
            req.id.clone(),
            super::add_element_comment::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::PaywallAddElementCommentQuote => RpcResponse::from(
            req.id.clone(),
            super::paywall_get_add_element_comment_quote::run_internal(&conf).await?,
        ),
        RpcMethod::PaywallAddElementComment => RpcResponse::from(
            req.id.clone(),
            super::paywall_add_element_comment::run_internal(params(req.params)?, &pool, &conf)
                .await?,
        ),
        RpcMethod::PaywallGetBoostElementQuote => RpcResponse::from(
            req.id.clone(),
            super::paywall_get_boost_element_quote::run_internal(&conf).await?,
        ),
        RpcMethod::PaywallBoostElement => RpcResponse::from(
            req.id.clone(),
            super::paywall_boost_element::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GenerateElementIssues => RpcResponse::from(
            req.id.clone(),
            super::generate_element_issues::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::SyncElements => RpcResponse::from(
            req.id.clone(),
            super::sync_elements::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GenerateElementIcons => RpcResponse::from(
            req.id.clone(),
            super::generate_element_icons::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GenerateElementCategories => RpcResponse::from(
            req.id.clone(),
            super::generate_element_categories::run_internal(params(req.params)?, &pool, &conf)
                .await?,
        ),
        RpcMethod::AddArea => RpcResponse::from(
            req.id.clone(),
            super::add_area::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GetArea => RpcResponse::from(
            req.id.clone(),
            super::get_area::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SetAreaTag => RpcResponse::from(
            req.id.clone(),
            super::set_area_tag::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::RemoveAreaTag => RpcResponse::from(
            req.id.clone(),
            super::remove_area_tag::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::SetAreaIcon => RpcResponse::from(
            req.id.clone(),
            super::set_area_icon::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::RemoveArea => RpcResponse::from(
            req.id.clone(),
            super::remove_area::run_internal(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GetTrendingCountries => RpcResponse::from(
            req.id.clone(),
            super::get_trending_countries::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetMostCommentedCountries => RpcResponse::from(
            req.id.clone(),
            super::get_most_commented_countries::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetTrendingCommunities => RpcResponse::from(
            req.id.clone(),
            super::get_trending_communities::run_internal(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GenerateAreasElementsMapping => RpcResponse::from(
            req.id.clone(),
            super::generate_areas_elements_mapping::run_internal(params(req.params)?, &pool, &conf)
                .await?,
        ),
    }?;
    Ok(Json(res))
}

fn params<T>(val: Option<Value>) -> Result<T>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_value(val.unwrap_or_default())?)
}

pub fn handle_rpc_error<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
    let (req, res) = res.into_parts();
    let error_message = res.error().unwrap().to_string();
    let body = RpcResponse::error(RpcError::server_error(Some(Value::String(error_message))));
    let res = res.set_body(serde_json::to_string(&body).unwrap());
    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();
    Ok(ErrorHandlerResponse::Response(res))
}
