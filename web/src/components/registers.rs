use leptos::prelude::*;

use common::emulator::{self, StateStoreFields};
use reactive_stores::Field;

pub fn register(label: String, value: u32, digits: usize) -> impl IntoView {
    let mask = (1 << (digits * 3)) - 1;

    view! {
        <div class = "register">
            <span class = "register-label"> {label} </span>
            <span class = "register-value"> {move || {format!("{:0digits$o}",  value & mask)}} </span>
        </div>
    }
}

#[component]
pub fn Registers(#[prop(into)] emulator: Field<emulator::State>) -> impl IntoView {
    view! {
        <div class = "flex-col" style = "align-items: end; gap: 4px;">
            { move ||  register(String::from("AU"), u32::from(emulator.au().get()), 6) }
            { move ||  register(String::from("AL"), u32::from(emulator.al().get()), 6) }
            <div class = "flex-row" style = "gap: 16px">
                { move ||  register(String::from("ICR"), u32::from(emulator.icr().get()), 1) }
                { move ||  register(String::from("SR"), u32::from(emulator.sr().get()), 2) }
            </div>
            { move ||  register(String::from("P"), u32::from(emulator.p().get()), 6) }
        </div>

    }
}
