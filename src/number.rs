use chumsky::prelude::*;

pub trait Number: Sized + Copy + Default {
    fn parser<'a>() -> impl Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> + Clone;
}

// impl<T: Number> Number for NumberExpr<T>
// where
//     T: core::ops::Div<Output = T>,
//     T: core::ops::Mul<Output = T>,
//     T: core::ops::Add<Output = T>,
//     T: core::ops::Sub<Output = T>,
// {
//     fn parser<'a>() -> impl Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> + Clone {
//         NumberExpr::<T>::parser()
//     }
// }

macro_rules! impl_signed_number {
    ($($t:ty),*) => {
        $(
        impl Number for $t {
            fn parser<'a>() -> impl Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> + Clone {
                just('-')
                    .or_not()
                    .then(text::int(10))
                    .to_slice()
                    .map(|v: &str| v.parse::<$t>().expect("Failed to parse signed number"))
            }
        }
        )*
    };
}
macro_rules! impl_unsigned_number {
    ($($t:ty),*) => {
        $(
        impl Number for $t {
            fn parser<'a>() -> impl Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> + Clone {
                text::int(10)
                    .to_slice()
                    .map(|v: &str| v.parse::<$t>().expect("Failed to parse unsigned number"))
            }
        }
        )*
    };
}
macro_rules! impl_floating_point {
    ($($t:ty),*) => {
        $(
        impl Number for $t {
            fn parser<'a>() -> impl Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> + Clone {
                let digits = text::digits(10);
                let frac = just('.').then(digits);
                let float = just('-')
                    .or_not()
                    .then(text::int(10).then(frac).or_not())
                    .to_slice()
                    .map(|input: &str| input.parse::<$t>().expect("Failed to parse as floating point number"));
                float
            }
        }
        )*
    };
}

impl_signed_number!(i8, i16, i32, i64, i128);
impl_unsigned_number!(u8, u16, u32, u64, u128);
impl_floating_point!(f32, f64);
