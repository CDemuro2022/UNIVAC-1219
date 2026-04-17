use crate::components::{
    assembly_editor::AssemblyEditor,
    controls::Controls,
    machine_state::{MachineState, MachineStateStoreFields},
    memory_inspector::MemoryInspector,
    terminal::Terminal,
};
use common::emulator::StateStoreFields;
use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use reactive_stores::Store;

use std::time::Duration;
use ux::{u18, u6};

#[component]
pub fn Home() -> impl IntoView {
    let machine_state = Store::new(MachineState::new());

    machine_state.emulator().p().set(u18::new(0o000540));
    machine_state.emulator().stop().set(u6::new(0b100000));

    let frame_micros = 1_000_000 / 60;
    let emulation_speed = 1;

    set_interval(
        move || {
            machine_state.update(|machine_state| {
                if machine_state.emulator.running {
                    machine_state.emulator.time_microseconds = 0;

                    while machine_state.emulator.time_microseconds < emulation_speed * frame_micros
                        && machine_state.emulator.running
                    {
                        machine_state.step(false);
                    }
                }
            });
        },
        Duration::from_micros(frame_micros),
    );

    let _handle = window_event_listener(ev::keypress, move |ev| {
        if let Ok(key_code) = u8::try_from(ev.key_code()) {
            match key_code {
                // When pressing enter, make sure to do a \n\r for the terminal
                13 => {
                    machine_state.update(|machine_state| {
                        machine_state.input_async(10);
                        machine_state.input_async(13)
                    });
                }
                key_code => {
                    machine_state.update(|machine_state| machine_state.input_async(key_code))
                }
            };
        }
    });

    view! {
        <ErrorBoundary fallback=|errors| {
            view! {
                <h1>"Uh oh! Something went wrong!"</h1>

                <p>"Errors: "</p>
                // Render a list of errors as strings - good for development purposes
                <ul>
                    {move || {
                        errors
                            .get()
                            .into_iter()
                            .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                            .collect_view()
                    }}

                </ul>
            }
        }>

        <div class="container">
            <h1 style="color:white">"Univac 1219"</h1>

            <div style="
                display: flex;
                flex-direction: row;
                column-gap: 16px;"
            >
                <AssemblyEditor machine_state />

                <Show
                    when=move || { machine_state.show_memory().get() }
                    fallback=|| view! {  }
                >
                    <MemoryInspector memory = machine_state.emulator().memory() p = machine_state.emulator().p() />
                </Show>

                <Controls machine_state />
                <Terminal terminal = machine_state.terminal() />
            </div>
        </div>
        </ErrorBoundary>
    }
}
