#![allow(unused_variables, unused_imports)] // Temporary, while in dev
mod test;
mod lesson1;
mod lesson2;
mod tourney;
mod empty;

pub use test::TestBot;
pub use lesson1::Lesson1Bot;
pub use lesson2::Lesson2Bot;
pub use tourney::TourneyV1Bot;
pub use empty::EmptyBot;