use crate::requests::AuthRedirectInfo;
use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use bld_core::logins::LoginProcess;
use tracing::{error, info};

const AUTH_REDIRECT_SUCCESS: &str =
    "Login completed, you can close this browser tab and go back to your terminal.";
const AUTH_REDIRECT_FAILED: &str = "An error occured while completing the login process.";

#[get("/authRedirect")]
pub async fn auth_redirect(
    info: Query<AuthRedirectInfo>,
    logins: Data<LoginProcess>,
) -> impl Responder {
    info!("Reached handler for /authRedirect route");
    let code = info.code.to_owned();
    let token = info.state.to_owned();
    match logins.code(token, code).await {
        Ok(_) => HttpResponse::Ok().body(AUTH_REDIRECT_SUCCESS),
        Err(e) => {
            error!("{e}");
            HttpResponse::Ok().body(AUTH_REDIRECT_FAILED)
        }
    }
}
