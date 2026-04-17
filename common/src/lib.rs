#[macro_export]
macro_rules! logger {
    () => {
        #[cfg(target_arch = "wasm32")]
        { }

        #[cfg(not(target_arch = "wasm32"))]
        eprintln!();
    };


    ($($t:tt)*) => {
        #[cfg(target_arch = "wasm32")]
        leptos::logging::log!($($t)*);

        #[cfg(not(target_arch = "wasm32"))]
        eprintln!($($t)*);
    };
}

pub mod arith;
pub mod assembler;
pub mod emulator;
pub mod instruction;
pub mod io;
pub mod memory;
pub mod tape;
