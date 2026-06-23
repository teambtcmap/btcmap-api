use crate::{
    db::{self, log::LogPool, main::user::schema::Role, main::MainPool},
    Result,
};
use actix_web::{
    dev::ServiceResponse,
    http::{
        header::{self},
        StatusCode,
    },
    middleware::ErrorHandlerResponse,
    post,
    web::{Data, Json},
    HttpRequest, HttpResponseBuilder,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use strum::VariantArray;

#[derive(Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: RpcMethod,
    pub params: Option<Value>,
    pub id: Value,
}

#[derive(Deserialize, PartialEq, Eq, VariantArray, Hash, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RpcMethod {
    // auth
    Signup,
    GetAdmin,
    ChangePassword,
    Signin,
    AddAdminAction,
    RemoveAdminAction,
    Whoami,
    GetApiKeys,
    RevokeApiKey,
    // element
    GetElement,
    SetElementTag,
    RemoveElementTag,
    BoostElement,
    AddElementComment,
    GenerateElementIssues,
    SyncElements,
    GenerateElementIcons,
    GenerateElementCategories,
    HumanizeOpeningHours,
    GetElementIssues,
    GenerateElementCommentCounts,
    // area
    AddArea,
    GetArea,
    SetAreaTag,
    RemoveAreaTag,
    SetAreaIcon,
    RemoveArea,
    GetTrendingCountries,
    GetTrendingCommunities,
    GenerateAreasElementsMapping,
    GenerateReports,
    GetAreaDashboard,
    GenerateAreaBboxes,
    // user
    GetUserActivity,
    SetUserTag,
    RemoveUserTag,
    GetMostActiveUsers,
    // invoice
    CreateInvoice,
    GetInvoice,
    SyncUnpaidInvoices,
    // search
    Search,
    // analytics
    GetReport,
    // Event management
    CreateEvent,
    GetEvents,
    GetEvent,
    DeleteEvent,
    // Import
    SubmitPlace,
    GetSubmittedPlace,
    RevokeSubmittedPlace,
    SyncSubmittedPlaces,
    // Matrix
    SendMatrixMessage,
    // Debug
    GetRequestLog,
    GetDailyInfraReport,
    GetTopClients,
    Dashboard,
}

impl Role {
    const ANON_METHODS: &[RpcMethod] = &[
        RpcMethod::Signup,
        RpcMethod::ChangePassword,
        RpcMethod::Signin,
    ];

    const USER_METHODS: &[RpcMethod] = &[
        RpcMethod::Whoami,
        RpcMethod::GetEvent,
        RpcMethod::GetApiKeys,
        RpcMethod::RevokeApiKey,
    ];

    const ADMIN_METHODS: &[RpcMethod] = &[
        // Admins can set and override custom place tags
        RpcMethod::SetElementTag,
        // Admins can remove custom place tags
        RpcMethod::RemoveElementTag,
        // Admins can create new areas
        RpcMethod::AddArea,
        // Admins can look up any area
        RpcMethod::GetArea,
        // Admins can set and override custom place tags
        RpcMethod::SetAreaTag,
        // Admins can remove custom area tags
        RpcMethod::RemoveAreaTag,
        // Admins can set and override area icons
        RpcMethod::SetAreaIcon,
        // Admins can remove any area
        RpcMethod::RemoveArea,
        // Admins can set and override custom user tags
        RpcMethod::SetUserTag,
        // Admins can remove custom user tags
        RpcMethod::RemoveUserTag,
        // Admins can request universal search
        RpcMethod::Search,
        // Admins can query user activity (TODO ask Rockedf if he still needs it)
        RpcMethod::GetUserActivity,
        // Admins can create events
        RpcMethod::CreateEvent,
        // Admins can retreive events
        RpcMethod::GetEvent,
        // Admins can import places
        RpcMethod::SubmitPlace,
        // Admins can revoke imported places
        RpcMethod::RevokeSubmittedPlace,
        // Admins can query place submissions by id
        RpcMethod::GetSubmittedPlace,
        // Admins can get daily infrastructure report
        RpcMethod::GetDailyInfraReport,
        // Admins can get top clients report
        RpcMethod::GetTopClients,
        // Admins can query the analytics dashboard
        RpcMethod::Dashboard,
    ];

    const PLACES_SOURCE_METHODS: &[RpcMethod] = &[
        RpcMethod::SubmitPlace,
        RpcMethod::RevokeSubmittedPlace,
        RpcMethod::GetSubmittedPlace,
    ];

    const EVENT_MANAGER_METHODS: &[RpcMethod] = &[
        RpcMethod::CreateEvent,
        RpcMethod::GetEvent,
        RpcMethod::DeleteEvent,
        RpcMethod::Search,
    ];

    const fn allowed_methods(&self) -> &[RpcMethod] {
        match self {
            Role::User => Self::USER_METHODS,
            Role::Admin => Self::ADMIN_METHODS,
            Role::Root => RpcMethod::VARIANTS,
            Role::PlacesSource => Self::PLACES_SOURCE_METHODS,
            Role::EventManager => Self::EVENT_MANAGER_METHODS,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: Value,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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

fn allowed_methods(roles: &[Role]) -> HashSet<RpcMethod> {
    let mut res = HashSet::new();
    // All anonymous methods are also acessible to authorized users
    for method in Role::ANON_METHODS {
        res.insert(method.clone());
    }
    for role in roles {
        for method in role.allowed_methods() {
            res.insert(method.clone());
        }
    }
    res
}

#[post("")]
pub async fn handle(
    req: HttpRequest,
    req_body: String,
    main_pool: Data<MainPool>,
    log_pool: Data<LogPool>,
) -> Result<Json<RpcResponse>> {
    let headers = req.headers();
    let Ok(req) = serde_json::from_str::<Map<String, Value>>(&req_body) else {
        let error_data = json!("Request body is not a valid JSON object");
        return Ok(Json(RpcResponse::error(RpcError::parse_error(Some(
            error_data,
        )))));
    };
    let Some(method) = req.get("method") else {
        let error_data = json!("Missing field: method");
        return Ok(Json(RpcResponse::error(RpcError::parse_error(Some(
            error_data,
        )))));
    };
    let Some(_) = method.as_str() else {
        let error_data = json!("Field method is not a string");
        return Ok(Json(RpcResponse::error(RpcError::parse_error(Some(
            error_data,
        )))));
    };
    let req: RpcRequest = match serde_json::from_value(Value::Object(req.clone())) {
        Ok(val) => val,
        Err(e) => {
            let data = Value::String(e.to_string());
            let e = RpcError::parse_error(Some(data));
            return Ok(Json(RpcResponse::error(e)));
        }
    };

    let bearer_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(String::from);

    if bearer_token.is_none() && !Role::ANON_METHODS.contains(&req.method) {
        return Ok(Json(RpcResponse::error(RpcError {
            code: 1,
            message: "Auth header is missing".to_string(),
            data: None,
        })));
    }

    let auth_token = match bearer_token {
        Some(bearer_token) => {
            let bearer_token =
                db::main::access_token::queries::select_by_secret(bearer_token, &main_pool).await?;
            let user =
                db::main::user::queries::select_by_id(bearer_token.user_id, &main_pool).await?;
            if bearer_token.roles.is_empty() {
                if !allowed_methods(&user.roles).contains(&req.method) {
                    return Ok(Json(RpcResponse::error(RpcError {
                        code: 1,
                        message: "You don't have permissions to call this method".to_string(),
                        data: None,
                    })));
                }
            } else if !allowed_methods(&bearer_token.roles).contains(&req.method) {
                return Ok(Json(RpcResponse::error(RpcError {
                    code: 1,
                    message: "You don't have permissions to call this method".to_string(),
                    data: None,
                })));
            }
            Some((bearer_token, user))
        }

        None => None,
    };

    let effective_roles = auth_token
        .as_ref()
        .map(|(token, user)| {
            if token.roles.is_empty() {
                user.roles.as_slice()
            } else {
                token.roles.as_slice()
            }
        })
        .unwrap_or(&[]);
    let user = auth_token.as_ref().map(|(_, user)| user);

    if req.jsonrpc != "2.0" {
        return Ok(Json(RpcResponse::invalid_request(Value::Null)));
    }

    let res: RpcResponse = match req.method {
        RpcMethod::Whoami => RpcResponse::from(
            req.id.clone(),
            super::auth::whoami::run(user.unwrap()).await?,
        ),
        RpcMethod::GetApiKeys => RpcResponse::from(
            req.id.clone(),
            super::auth::get_api_keys::run(user.unwrap(), &main_pool).await?,
        ),
        RpcMethod::RevokeApiKey => RpcResponse::from(
            req.id.clone(),
            super::auth::revoke_api_key::run(params(req.params)?, user.unwrap(), &main_pool)
                .await?,
        ),
        // element
        RpcMethod::GetElement => RpcResponse::from(
            req.id.clone(),
            super::element::get_element::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::SetElementTag => RpcResponse::from(
            req.id.clone(),
            super::set_element_tag::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::RemoveElementTag => RpcResponse::from(
            req.id.clone(),
            super::remove_element_tag::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::BoostElement => RpcResponse::from(
            req.id.clone(),
            super::boost_element::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::AddElementComment => RpcResponse::from(
            req.id.clone(),
            super::add_element_comment::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GenerateElementIssues => RpcResponse::from(
            req.id.clone(),
            super::generate_element_issues::run(&main_pool).await?,
        ),
        RpcMethod::SyncElements => RpcResponse::from(
            req.id.clone(),
            super::sync_elements::run(&main_pool, &log_pool).await?,
        ),
        RpcMethod::GenerateElementIcons => RpcResponse::from(
            req.id.clone(),
            super::generate_element_icons::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GenerateElementCategories => RpcResponse::from(
            req.id.clone(),
            super::generate_element_categories::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::HumanizeOpeningHours => RpcResponse::from(
            req.id.clone(),
            super::humanize_opening_hours::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetElementIssues => RpcResponse::from(
            req.id.clone(),
            super::get_element_issues::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GenerateElementCommentCounts => RpcResponse::from(
            req.id.clone(),
            super::element::generate_element_comment_counts::run(&main_pool).await?,
        ),
        // area
        RpcMethod::AddArea => RpcResponse::from(
            req.id.clone(),
            super::area::add_area::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetArea => RpcResponse::from(
            req.id.clone(),
            super::get_area::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::SetAreaTag => RpcResponse::from(
            req.id.clone(),
            super::set_area_tag::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::RemoveAreaTag => RpcResponse::from(
            req.id.clone(),
            super::remove_area_tag::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::SetAreaIcon => RpcResponse::from(
            req.id.clone(),
            super::set_area_icon::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::RemoveArea => RpcResponse::from(
            req.id.clone(),
            super::remove_area::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetTrendingCountries => RpcResponse::from(
            req.id.clone(),
            super::get_trending_countries::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetTrendingCommunities => RpcResponse::from(
            req.id.clone(),
            super::get_trending_communities::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GenerateAreasElementsMapping => RpcResponse::from(
            req.id.clone(),
            super::generate_areas_elements_mapping::run(&main_pool).await?,
        ),
        RpcMethod::GenerateReports => RpcResponse::from(
            req.id.clone(),
            super::generate_reports::run(&main_pool).await?,
        ),
        RpcMethod::GetAreaDashboard => RpcResponse::from(
            req.id.clone(),
            super::get_area_dashboard::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetUserActivity => RpcResponse::from(
            req.id.clone(),
            super::get_user_activity::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::SetUserTag => RpcResponse::from(
            req.id.clone(),
            super::set_user_tag::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::RemoveUserTag => RpcResponse::from(
            req.id.clone(),
            super::remove_user_tag::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetMostActiveUsers => RpcResponse::from(
            req.id.clone(),
            super::get_most_active_users::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GenerateAreaBboxes => RpcResponse::from(
            req.id.clone(),
            super::area::generate_bboxes::run(&main_pool).await?,
        ),
        // auth
        RpcMethod::Signin => RpcResponse::from(
            req.id.clone(),
            super::auth::signin::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::Signup => RpcResponse::from(
            req.id.clone(),
            super::auth::signup::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetAdmin => RpcResponse::from(
            req.id.clone(),
            super::admin::get_admin::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::ChangePassword => RpcResponse::from(
            req.id.clone(),
            super::auth::change_password::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::AddAdminAction => RpcResponse::from(
            req.id.clone(),
            super::admin::add_admin_action::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::RemoveAdminAction => RpcResponse::from(
            req.id.clone(),
            super::admin::remove_admin_action::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetInvoice => RpcResponse::from(
            req.id.clone(),
            super::invoice::get_invoice::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::CreateInvoice => RpcResponse::from(
            req.id.clone(),
            super::invoice::create_invoice::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::SyncUnpaidInvoices => RpcResponse::from(
            req.id.clone(),
            super::sync_unpaid_invoices::run(&main_pool).await?,
        ),
        RpcMethod::Search => RpcResponse::from(
            req.id.clone(),
            super::search::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetReport => RpcResponse::from(
            req.id.clone(),
            super::analytics::get_report::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::CreateEvent => RpcResponse::from(
            req.id.clone(),
            super::event::create_event::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::GetEvents => RpcResponse::from(
            req.id.clone(),
            super::event::get_events::run(&main_pool).await?,
        ),
        RpcMethod::GetEvent => RpcResponse::from(
            req.id.clone(),
            super::event::get_event::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::DeleteEvent => RpcResponse::from(
            req.id.clone(),
            super::event::delete_event::run(params(req.params)?, &main_pool).await?,
        ),
        RpcMethod::SubmitPlace => {
            let params: super::import::submit_place::Params = params(req.params)?;
            let token = &auth_token.as_ref().unwrap().0;
            super::import::ensure_can_access_origin(effective_roles, token, &params.origin)?;
            RpcResponse::from(
                req.id.clone(),
                super::import::submit_place::run(params, &main_pool).await?,
            )
        }
        RpcMethod::GetSubmittedPlace => {
            let params: super::import::get_submitted_place::Params = params(req.params)?;
            let token = &auth_token.as_ref().unwrap().0;
            RpcResponse::from(
                req.id.clone(),
                super::import::get_submitted_place::run(params, effective_roles, token, &main_pool)
                    .await?,
            )
        }
        RpcMethod::RevokeSubmittedPlace => {
            let params: super::import::revoke_submitted_place::Params = params(req.params)?;
            let token = &auth_token.as_ref().unwrap().0;
            RpcResponse::from(
                req.id.clone(),
                super::import::revoke_submitted_place::run(
                    params,
                    effective_roles,
                    token,
                    &main_pool,
                )
                .await?,
            )
        }
        RpcMethod::SyncSubmittedPlaces => RpcResponse::from(
            req.id.clone(),
            super::import::sync_submitted_places::run(&main_pool).await?,
        ),
        RpcMethod::SendMatrixMessage => {
            super::matrix::send_matrix_message::run(params(req.params)?, &main_pool).await;
            Ok(RpcResponse::success(
                req.id.clone(),
                serde_json::Value::Null,
            ))
        }
        RpcMethod::GetRequestLog => RpcResponse::from(
            req.id.clone(),
            super::analytics::get_request_log::run(params(req.params)?, &log_pool).await?,
        ),
        RpcMethod::GetDailyInfraReport => RpcResponse::from(
            req.id.clone(),
            super::analytics::get_daily_infra_report::run(&log_pool, &main_pool).await?,
        ),
        RpcMethod::GetTopClients => RpcResponse::from(
            req.id.clone(),
            super::analytics::get_top_clients::run(&log_pool).await?,
        ),
        RpcMethod::Dashboard => RpcResponse::from(
            req.id.clone(),
            super::analytics::dashboard::run(&main_pool, &log_pool).await?,
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
    let body = serde_json::to_string(&body).unwrap();
    let res = HttpResponseBuilder::new(StatusCode::OK).body(body);
    let res = ServiceResponse::new(req, res)
        .map_into_boxed_body()
        .map_into_right_body();
    Ok(ErrorHandlerResponse::Response(res))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        db::log::test::pool as log_pool, db::main::test::pool, service::overpass::OverpassElement,
    };
    use actix_web::{
        http::{header, StatusCode},
        test,
        web::scope,
        App,
    };
    use matrix_sdk::Client;
    use serde_json::json;

    #[test]
    async fn invalid_json() {
        let pool = pool();
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_payload("not json")
            .to_request();

        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::OK);

        let body: RpcResponse = test::read_body_json(res).await;
        assert_eq!(body.error.unwrap().code, -32700); // Parse error
    }

    #[test]
    async fn missing_method() {
        let pool = pool();
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "2.0",
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.error.unwrap().code, -32700); // Parse error
    }

    #[test]
    async fn anonymous_method_allowed() -> Result<()> {
        let pool = pool();
        db::main::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "signup",
                "params": {"username": "satoshi", "password": "ihsotasatoshi123"},
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.error, None);
        Ok(())
    }

    #[test]
    async fn protected_method_without_auth() -> Result<()> {
        let pool = pool();
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "add_area",
                "params": {"name": "test"},
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert!(res.error.is_some());
        Ok(())
    }

    #[test]
    async fn invalid_jsonrpc_version() {
        let pool = pool();
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "1.0",
                "method": "signup",
                "params": {"username": "satoshi", "password": "ihsotasatoshi123"},
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.error.unwrap().code, -32600); // Invalid Request
    }

    #[test]
    async fn valid_request_with_auth() -> Result<()> {
        let pool = pool();
        let user = db::main::user::queries::insert("root", "", &pool).await?;
        let _token = db::main::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer secret"))
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "whoami",
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.error, None);
        Ok(())
    }

    #[test]
    async fn get_submitted_place_id_origin_bypass_blocked() -> Result<()> {
        let pool = pool();

        // Insert a square submission
        let square_submission = db::main::place_submission::blocking_queries::InsertArgs {
            origin: "square".to_string(),
            external_id: "123".to_string(),
            lat: 1.0,
            lon: 2.0,
            category: "test".to_string(),
            name: "Square Place".to_string(),
            extra_fields: serde_json::Map::new(),
        };
        db::main::place_submission::queries::insert(square_submission, &pool).await?;

        // Create a user with PlacesSource role and a token scoped to coinos
        let user = db::main::user::queries::insert("source_user", "", &pool).await?;
        let _token = db::main::access_token::queries::insert_with_import_origins(
            user.id,
            "".to_string(),
            "scoped_secret".to_string(),
            vec![Role::PlacesSource],
            vec!["coinos".to_string()],
            &pool,
        )
        .await?;

        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        // Try to access square submission by id, but supply coinos origin to bypass pre-check
        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer scoped_secret"))
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "get_submitted_place",
                "params": {"id": 1, "origin": "coinos"},
                "id": 1
            }))
            .to_request();

        let res = test::call_service(&app, req).await;
        let body = test::read_body(res).await;
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("coinos") || body_str.contains("not allowed"),
            "should have rejected access to square submission with coinos-scoped token; body: {body_str}"
        );
        Ok(())
    }

    #[test]
    async fn revoke_submitted_place_id_origin_bypass_blocked() -> Result<()> {
        let pool = pool();

        // Insert a square submission
        let square_submission = db::main::place_submission::blocking_queries::InsertArgs {
            origin: "square".to_string(),
            external_id: "123".to_string(),
            lat: 1.0,
            lon: 2.0,
            category: "test".to_string(),
            name: "Square Place".to_string(),
            extra_fields: serde_json::Map::new(),
        };
        db::main::place_submission::queries::insert(square_submission, &pool).await?;

        // Create a user with PlacesSource role and a token scoped to coinos
        let user = db::main::user::queries::insert("source_user", "", &pool).await?;
        let _token = db::main::access_token::queries::insert_with_import_origins(
            user.id,
            "".to_string(),
            "scoped_secret".to_string(),
            vec![Role::PlacesSource],
            vec!["coinos".to_string()],
            &pool,
        )
        .await?;

        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool.clone()))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        // Try to revoke square submission by id, but supply coinos origin to bypass pre-check
        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "Bearer scoped_secret"))
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "revoke_submitted_place",
                "params": {"id": 1, "origin": "coinos"},
                "id": 1
            }))
            .to_request();

        let res = test::call_service(&app, req).await;
        let body = test::read_body(res).await;
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("coinos") || body_str.contains("not allowed"),
            "should have rejected revocation of square submission with coinos-scoped token; body: {body_str}"
        );

        // Verify the submission was NOT revoked
        let submission = db::main::place_submission::queries::select_by_id(1, &pool).await?;
        assert!(
            !submission.revoked,
            "submission should not have been revoked"
        );

        Ok(())
    }

    #[test]
    async fn unauthorized_method() {
        let pool = pool();
        let client: Option<Client> = None;
        let log_pool = log_pool();
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(client))
                .app_data(Data::new(log_pool))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "add_area",  // Requires admin role
                "params": {"name": "test"},
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert!(res.error.is_some());
    }
}
