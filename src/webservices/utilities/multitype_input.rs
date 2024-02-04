use actix_web::{web, Either};

pub(crate) type EitherInputExtended<Left, Right> =
    Either<web::Form<Left>, Either<web::Json<Left>, actix_multipart::form::MultipartForm<Right>>>;

pub(crate) struct EitherInputExtendedWrapper<Left, Right>(pub EitherInputExtended<Left, Right>)
where
    Right: actix_multipart::form::MultipartCollect;

impl<'a, Left, Right> From<&'a EitherInputExtendedWrapper<Left, Right>>
    for Either<&'a Left, &'a Right>
where
    Right: actix_multipart::form::MultipartCollect,
{
    fn from(value: &'a EitherInputExtendedWrapper<Left, Right>) -> Self {
        match value.0 {
            Either::Left(ref left) => Either::Left(left),
            Either::Right(ref right) => match right {
                Either::Left(ref left) => Either::Left(left),
                Either::Right(ref right) => Either::Right(right),
            },
        }
    }
}

