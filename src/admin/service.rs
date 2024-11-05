use super::Admin;
use crate::Result;
use crate::{discord, Error};
use deadpool_sqlite::Pool;

//#[cfg(test)]
//pub async fn mock_admin(password: &str, pool: &Pool) -> Admin {
//    let password = password.to_string();
//    pool.get()
//        .await
//        .unwrap()
//        .interact(move |conn| Admin::insert("test", &password, conn))
//        .await
//        .unwrap()
//        .unwrap()
//        .unwrap()
//}

pub async fn check_rpc(password: &str, action: &str, pool: &Pool) -> Result<Admin> {
    let password = password.to_string();
    let admin = pool
        .get()
        .await?
        .interact(move |conn| Admin::select_by_password(&password, conn))
        .await??
        .unwrap();
    if !admin.allowed_actions.contains(&"all".into())
        && !admin.allowed_actions.contains(&action.into())
    {
        let log_message = format!(
            "{} tried to call action {} without proper permissions",
            admin.name, action,
        );
        discord::send_message_to_channel(&log_message, discord::CHANNEL_API).await;
        Err(Error::Unauthorized(format!(
            "You are not allowed to perform this action"
        )))?
    }
    Ok(admin)
}

#[cfg(test)]
mod tests {
    use crate::Result;

    #[actix_web::test]
    async fn no_header() -> Result<()> {
        //let state = mock_state().await;
        //super::mock_admin("test", &state.pool).await;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::new(state.pool))
        //        .service(scope("/").service(get)),
        //)
        //.await;
        //let req = TestRequest::get().uri("/").to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(401, res.status().as_u16());
        Ok(())
    }

    #[actix_web::test]
    async fn valid_token() -> Result<()> {
        //let state = mock_state().await;
        //super::mock_admin("test", &state.pool).await;
        //let app = test::init_service(
        //    App::new()
        //        .app_data(Data::new(state.pool))
        //        .service(scope("/").service(get)),
        //)
        //.await;
        //let req = TestRequest::get()
        //    .uri("/")
        //    .append_header(("Authorization", "Bearer test"))
        //    .to_request();
        //let res = test::call_service(&app, req).await;
        //assert_eq!(200, res.status().as_u16());
        Ok(())
    }

    //#[get("")]
    //async fn get(req: HttpRequest, pool: Data<Pool>) -> Result<impl Responder, Error> {
    //    super::check(&req, &pool).await?;
    //    Ok(Response::ok())
    //}
}
