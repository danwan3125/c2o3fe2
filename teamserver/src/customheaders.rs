use axum::{
    extract::FromRequestParts, http::{StatusCode, header::{AUTHORIZATION, COOKIE, HeaderValue, USER_AGENT}, request::Parts},
};
pub struct ExtractAuthorizationToken(pub HeaderValue);

impl<S> FromRequestParts<S> for ExtractAuthorizationToken
where 
    S:Send+Sync,
{
    type Rejection = (StatusCode,&'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection>
    {
        println!("Received:{:?}",parts.headers);
        if let Some(auth_token)=parts.headers.get(AUTHORIZATION){
            Ok(ExtractAuthorizationToken(auth_token.clone()))
        }
        else{
            Err((StatusCode::UNAUTHORIZED,"Authorization token is missing."))
        }
    }
}

pub struct ExtractCookie(pub HeaderValue);

impl<S> FromRequestParts<S> for ExtractCookie
where 
    S:Send+Sync,
{
    type Rejection = (StatusCode,&'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection>
    {
        if let Some(user_agent)=parts.headers.get(COOKIE){
            Ok(ExtractCookie(user_agent.clone()))
        }
        else{
            Err((StatusCode::UNAUTHORIZED,"Authorization token is missing."))
        }
    }
}