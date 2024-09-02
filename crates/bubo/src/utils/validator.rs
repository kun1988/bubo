use axum::{async_trait, extract::{rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection}, FromRequest, Path, Query, Request}, Form, Json};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::error::{BuboError, BuboResult};


#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedForm<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Form<T>: FromRequest<S, Rejection = FormRejection>,
{
    type Rejection = BuboError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedForm(value))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonValid<T>(pub T);
#[derive(Debug, Clone, Copy, Default)]
pub struct FormValid<T>(pub T);
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryValid<T>(pub T);
#[derive(Debug, Clone, Copy, Default)]
pub struct PathValid<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for JsonValid<T>
    where
        T: DeserializeOwned + Validate,
        S: Send + Sync,
        Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = BuboError;

    async fn from_request(req: Request, state: &S) -> BuboResult<Self> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(JsonValid(value))
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for FormValid<T>
    where
        T: DeserializeOwned + Validate,
        S: Send + Sync,
        Form<T>: FromRequest<S, Rejection = FormRejection>,
{
    type Rejection = BuboError;

    async fn from_request(req: Request, state: &S) -> BuboResult<Self> {
        let Form(value) = Form::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(FormValid(value))
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for QueryValid<T>
    where
        T: DeserializeOwned + Validate,
        S: Send + Sync,
        Query<T>: FromRequest<S, Rejection = QueryRejection>,
{
    type Rejection = BuboError;

    async fn from_request(req: Request, state: &S) -> BuboResult<Self> {
        let Query(value) = Query::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(QueryValid(value))
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for PathValid<T>
    where
        T: DeserializeOwned + Validate,
        S: Send + Sync,
        Path<T>: FromRequest<S, Rejection = PathRejection>,
{
    type Rejection = BuboError;

    async fn from_request(req: Request, state: &S) -> BuboResult<Self> {
        let Path(value) = Path::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(PathValid(value))
    }
}