#![allow(unused_variables)]
pub mod engine;
pub mod skeleton;

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug_println {
    ( $($args : expr),* ) => { println! ( $( $args ),* ) };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug_println {
    ( $($args : expr),* ) => { };
}