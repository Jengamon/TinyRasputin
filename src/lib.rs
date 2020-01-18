#![allow(unused_variables)]
pub mod engine;
pub mod skeleton;

#[cfg(feature = "debug_print")]
#[macro_export]
macro_rules! debug_println {
    ( $($args : expr),* ) => { println! ( $( $args ),* ) };
}

#[cfg(not(feature = "debug_print"))]
#[macro_export]
macro_rules! debug_println {
    ( $($args : expr),* ) => { print!("") };
}
