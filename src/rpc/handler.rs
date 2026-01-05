use crate::{
    db::{
        self,
        conf::schema::Conf,
        user::schema::{Role, User},
    },
    service, Result,
};
use actix_web::{
    dev::ServiceResponse,
    http::{
        header::{self, HeaderMap},
        StatusCode,
    },
    middleware::ErrorHandlerResponse,
    post,
    web::{Data, Json},
    HttpRequest, HttpResponseBuilder,
};
use deadpool_sqlite::Pool;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use strum::VariantArray;
use time::OffsetDateTime;

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
    AddUser,
    GetAdmin,
    ChangePassword,
    CreateApiKey,
    AddAdminAction,
    RemoveAdminAction,
    Whoami,
    // element
    GetElement,
    SetElementTag,
    RemoveElementTag,
    BoostElement,
    PaywallGetBoostElementQuote,
    PaywallBoostElement,
    AddElementComment,
    PaywallGetAddElementCommentQuote,
    PaywallAddElementComment,
    GenerateElementIssues,
    SyncElements,
    GenerateElementIcons,
    GenerateElementCategories,
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
}

impl Role {
    const ANON_METHODS: &[RpcMethod] = &[
        RpcMethod::Search,
        RpcMethod::AddUser,
        RpcMethod::ChangePassword,
        RpcMethod::CreateApiKey,
        // Anons can use some paid features so they need to be able to check their invoice status
        RpcMethod::GetInvoice,
        // TODO consider making private
        RpcMethod::GetElement,
        // Android uses that anonymously, we should keep it public
        RpcMethod::PaywallGetAddElementCommentQuote,
        RpcMethod::PaywallAddElementComment,
        // Android uses that anonymously, we should keep it public
        RpcMethod::PaywallGetBoostElementQuote,
        RpcMethod::PaywallBoostElement,
        // Used by our website, we need to create website user or stop using those methods
        RpcMethod::GetElementIssues,
        RpcMethod::GetAreaDashboard,
        RpcMethod::GetMostActiveUsers,
    ];

    const USER_METHODS: &[RpcMethod] = &[RpcMethod::Whoami, RpcMethod::GetEvent];

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
    ];

    const PLACES_SOURCE_METHODS: &[RpcMethod] = &[
        RpcMethod::SubmitPlace,
        RpcMethod::RevokeSubmittedPlace,
        RpcMethod::GetSubmittedPlace,
    ];

    const EVENT_MANAGER_METHODS: &[RpcMethod] = &[
        RpcMethod::CreateEvent,
        RpcMethod::GetEvent,
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
    pool: Data<Pool>,
    conf: Data<Conf>,
) -> Result<Json<RpcResponse>> {
    let conn_info = req.connection_info();
    let req_ip = conn_info.realip_remote_addr().unwrap_or("0.0.0.0");

    let started_at = OffsetDateTime::now_utc();
    let matrix_client = service::matrix::client(&pool).await;
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

    let mut access_token = extract_access_token(headers, &req.params);
    // TODO remove
    if req.method == RpcMethod::AddUser || req.method == RpcMethod::CreateApiKey {
        access_token.clear();
    }

    if access_token.is_empty() && !Role::ANON_METHODS.contains(&req.method) {
        return Ok(Json(RpcResponse::error(RpcError {
            code: 1,
            message: "Auth header is missing".into(),
            data: None,
        })));
    }

    let user: Option<User> = if access_token.is_empty() {
        None
    } else {
        let access_token = db::access_token::queries::select_by_secret(access_token, &pool).await?;
        let user = db::user::queries::select_by_id(access_token.user_id, &pool).await?;
        if access_token.roles.is_empty() {
            if !allowed_methods(&user.roles).contains(&req.method) {
                return Ok(Json(RpcResponse::error(RpcError {
                    code: 1,
                    message: "You don't have permissions to call this method".into(),
                    data: None,
                })));
            }
        } else {
            if !allowed_methods(&access_token.roles).contains(&req.method) {
                return Ok(Json(RpcResponse::error(RpcError {
                    code: 1,
                    message: "You don't have permissions to call this method".into(),
                    data: None,
                })));
            }
        }
        Some(user)
    };
    let user_id = user.as_ref().map(|it| it.id);

    if req.jsonrpc != "2.0" {
        return Ok(Json(RpcResponse::invalid_request(Value::Null)));
    }

    let req_params = req.params.clone();

    let res: RpcResponse = match req.method {
        RpcMethod::Whoami => RpcResponse::from(
            req.id.clone(),
            super::auth::whoami::run(&user.unwrap()).await?,
        ),
        // element
        RpcMethod::GetElement => RpcResponse::from(
            req.id.clone(),
            super::element::get_element::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SetElementTag => RpcResponse::from(
            req.id.clone(),
            super::set_element_tag::run(params(req.params)?, &user.unwrap(), &pool, &conf).await?,
        ),
        RpcMethod::RemoveElementTag => RpcResponse::from(
            req.id.clone(),
            super::remove_element_tag::run(params(req.params)?, &user.unwrap(), &pool, &conf)
                .await?,
        ),
        RpcMethod::BoostElement => RpcResponse::from(
            req.id.clone(),
            super::boost_element::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::PaywallGetBoostElementQuote => RpcResponse::from(
            req.id.clone(),
            super::paywall_get_boost_element_quote::run(&conf).await?,
        ),
        RpcMethod::PaywallBoostElement => RpcResponse::from(
            req.id.clone(),
            super::paywall_boost_element::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::AddElementComment => RpcResponse::from(
            req.id.clone(),
            super::add_element_comment::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::PaywallGetAddElementCommentQuote => RpcResponse::from(
            req.id.clone(),
            super::paywall_get_add_element_comment_quote::run(&conf).await?,
        ),
        RpcMethod::PaywallAddElementComment => RpcResponse::from(
            req.id.clone(),
            super::paywall_add_element_comment::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GenerateElementIssues => RpcResponse::from(
            req.id.clone(),
            super::generate_element_issues::run(&pool).await?,
        ),
        RpcMethod::SyncElements => RpcResponse::from(
            req.id.clone(),
            super::sync_elements::run(&user.unwrap(), &pool, &conf, matrix_client).await?,
        ),
        RpcMethod::GenerateElementIcons => RpcResponse::from(
            req.id.clone(),
            super::generate_element_icons::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GenerateElementCategories => RpcResponse::from(
            req.id.clone(),
            super::generate_element_categories::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetElementIssues => RpcResponse::from(
            req.id.clone(),
            super::get_element_issues::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GenerateElementCommentCounts => RpcResponse::from(
            req.id.clone(),
            super::element::generate_element_comment_counts::run(&pool).await?,
        ),
        // area
        RpcMethod::AddArea => RpcResponse::from(
            req.id.clone(),
            super::add_area::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetArea => RpcResponse::from(
            req.id.clone(),
            super::get_area::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SetAreaTag => RpcResponse::from(
            req.id.clone(),
            super::set_area_tag::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::RemoveAreaTag => RpcResponse::from(
            req.id.clone(),
            super::remove_area_tag::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SetAreaIcon => RpcResponse::from(
            req.id.clone(),
            super::set_area_icon::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::RemoveArea => RpcResponse::from(
            req.id.clone(),
            super::remove_area::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetTrendingCountries => RpcResponse::from(
            req.id.clone(),
            super::get_trending_countries::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetTrendingCommunities => RpcResponse::from(
            req.id.clone(),
            super::get_trending_communities::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GenerateAreasElementsMapping => RpcResponse::from(
            req.id.clone(),
            super::generate_areas_elements_mapping::run(&pool).await?,
        ),
        RpcMethod::GenerateReports => {
            RpcResponse::from(req.id.clone(), super::generate_reports::run(&pool).await?)
        }
        RpcMethod::GetAreaDashboard => RpcResponse::from(
            req.id.clone(),
            super::get_area_dashboard::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetUserActivity => RpcResponse::from(
            req.id.clone(),
            super::get_user_activity::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SetUserTag => RpcResponse::from(
            req.id.clone(),
            super::set_user_tag::run(params(req.params)?, &user.unwrap(), &pool, &conf).await?,
        ),
        RpcMethod::RemoveUserTag => RpcResponse::from(
            req.id.clone(),
            super::remove_user_tag::run(params(req.params)?, &user.unwrap(), &pool, &conf).await?,
        ),
        RpcMethod::GetMostActiveUsers => RpcResponse::from(
            req.id.clone(),
            super::get_most_active_users::run(params(req.params)?, &pool).await?,
        ),
        // auth
        RpcMethod::CreateApiKey => RpcResponse::from(
            req.id.clone(),
            super::auth::create_api_key::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::AddUser => RpcResponse::from(
            req.id.clone(),
            super::auth::add_user::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GetAdmin => RpcResponse::from(
            req.id.clone(),
            super::admin::get_admin::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::ChangePassword => RpcResponse::from(
            req.id.clone(),
            super::auth::change_password::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::AddAdminAction => RpcResponse::from(
            req.id.clone(),
            super::admin::add_admin_action::run(params(req.params)?, &user.unwrap(), &pool, &conf)
                .await?,
        ),
        RpcMethod::RemoveAdminAction => RpcResponse::from(
            req.id.clone(),
            super::admin::remove_admin_action::run(
                params(req.params)?,
                &user.unwrap(),
                &pool,
                &conf,
            )
            .await?,
        ),
        RpcMethod::GetInvoice => RpcResponse::from(
            req.id.clone(),
            super::invoice::get_invoice::run(params(req.params)?, &pool, matrix_client).await?,
        ),
        RpcMethod::CreateInvoice => RpcResponse::from(
            req.id.clone(),
            super::invoice::create_invoice::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SyncUnpaidInvoices => RpcResponse::from(
            req.id.clone(),
            super::sync_unpaid_invoices::run(&pool, matrix_client).await?,
        ),
        RpcMethod::Search => RpcResponse::from(
            req.id.clone(),
            super::search::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetReport => RpcResponse::from(
            req.id.clone(),
            super::analytics::get_report::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::CreateEvent => RpcResponse::from(
            req.id.clone(),
            super::event::create_event::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::GetEvents => {
            RpcResponse::from(req.id.clone(), super::event::get_events::run(&pool).await?)
        }
        RpcMethod::GetEvent => RpcResponse::from(
            req.id.clone(),
            super::event::get_event::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::DeleteEvent => RpcResponse::from(
            req.id.clone(),
            super::event::delete_event::run(params(req.params)?, &pool, &conf).await?,
        ),
        RpcMethod::SubmitPlace => RpcResponse::from(
            req.id.clone(),
            super::import::submit_place::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::RevokeSubmittedPlace => RpcResponse::from(
            req.id.clone(),
            super::import::revoke_submitted_place::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::GetSubmittedPlace => RpcResponse::from(
            req.id.clone(),
            super::import::get_submitted_place::run(params(req.params)?, &pool).await?,
        ),
        RpcMethod::SyncSubmittedPlaces => RpcResponse::from(
            req.id.clone(),
            super::import::sync_submitted_places::run(&pool, &matrix_client).await?,
        ),
        RpcMethod::SendMatrixMessage => RpcResponse::from(
            req.id.clone(),
            super::matrix::send_matrix_message::run(params(req.params)?, &matrix_client).await,
        ),
    }?;

    if let Some(user_id) = user_id {
        db::rpc_call::queries::insert(
            user_id,
            req_ip.to_string(),
            method.as_str().unwrap_or_default().to_string(),
            req_params.map(|it| it.as_object().map(|it| it.clone()).unwrap()),
            started_at,
            OffsetDateTime::now_utc(),
            &pool,
        )
        .await?;
    }

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

fn extract_access_token(headers: &HeaderMap, params: &Option<Value>) -> String {
    if headers.contains_key(header::AUTHORIZATION) {
        let header = headers
            .get(header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap_or_default();
        return header.replace("Bearer ", "");
    }
    let Some(params) = params else {
        return "".into();
    };
    let Some(password) = params.get("password") else {
        return "".into();
    };
    let Some(password) = password.as_str() else {
        return "".into();
    };
    password.into()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{db::test::pool, service::overpass::OverpassElement};
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
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
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
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
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
        db::element::queries::insert(OverpassElement::mock(1), &pool).await?;
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "2.0",
                "method": "get_element",
                "params": {"id": 1},
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
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
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
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&json!({
                "jsonrpc": "1.0",
                "method": "get_element",
                "params": {"element_id": 1},
                "id": 1
            }))
            .to_request();

        let res: RpcResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.error.unwrap().code, -32600); // Invalid Request
    }

    #[test]
    async fn valid_request_with_auth() -> Result<()> {
        let pool = pool();
        let user = db::user::queries::insert("root", "", &pool).await?;
        let _token = db::access_token::queries::insert(
            user.id,
            "".into(),
            "secret".into(),
            vec![Role::Root],
            &pool,
        )
        .await?;
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
                .service(scope("/").service(super::handle)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header((header::AUTHORIZATION, "secret"))
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
    async fn unauthorized_method() {
        let pool = pool();
        let conf = Conf::mock();
        let client: Option<Client> = None;
        let app = test::init_service(
            App::new()
                .app_data(Data::new(pool))
                .app_data(Data::new(conf))
                .app_data(Data::new(client))
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

    #[test]
    async fn test_extract_access_token_from_header() {
        let headers = header::HeaderMap::new();
        let mut headers_with_auth = header::HeaderMap::new();
        headers_with_auth.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_static("Bearer test_token"),
        );

        // Test with no headers
        assert_eq!(extract_access_token(&headers, &None), "");

        // Test with auth header
        assert_eq!(
            extract_access_token(&headers_with_auth, &None),
            "test_token"
        );

        // Test with params fallback
        let params = Some(json!({"password": "param_token"}));
        assert_eq!(extract_access_token(&headers, &params), "param_token");
    }
}
