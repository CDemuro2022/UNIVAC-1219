use crate::components::machine_state::MachineState;
use crate::components::machine_state::MachineStateStoreFields;
use crate::components::registers::Registers;
use crate::components::tape_input::TapeInput;
use common::emulator::StateStoreFields;

use leptos::prelude::*;
use reactive_stores::{Field, Store};
use ux::{u18, u3, u5, u6};

#[component]
pub fn StopButton(
    index: usize,
    #[prop(into)] flags: Field<u6>,
    #[prop(into)] stopped_flags: Field<u6>,
) -> impl IntoView {
    let mask = u6::new(1 << index);

    view! {
        <button
            on:click=move |_| { flags.update(|flags| *flags = *flags ^ mask ); }
            class = move ||
                if (stopped_flags.get() & mask) != u6::new(0) {
                    "toggle toggle-red"
                }
                else if (flags.get() & mask) != u6::new(0) {
                    "toggle toggle-on"
                } else {
                    "toggle toggle-off"
                }
        >
            {index}
        </button>
    }
}

#[component]
pub fn SkipButton(index: usize, #[prop(into)] flags: Field<u5>) -> impl IntoView {
    let mask = u5::new(1 << index);
    view! {
        <button
            on:click=move |_| { flags.update(|flags| *flags = *flags ^ mask ); }
            class = move ||  if (flags.get() & mask) != u5::new(0) { "toggle toggle-on" } else { "toggle toggle-off" }
        >
            {index}
        </button>
    }
}

#[component]
pub fn Controls(#[prop(into)] machine_state: Store<MachineState>) -> impl IntoView {
    view! {
        <div class = "pane flex-col" style = "justify-content: space-between;">
            <Registers emulator = machine_state.emulator() />
            <div class = "flex-col" style = "gap: 8px" >

                <TapeInput machine_state />

                <div class = "subpane flex-row" style = "gap: 4px; justify-content: end">
                    <button class = "toggle"
                        on:click=move |_| {
                            machine_state.show_memory().update(|show_memory| *show_memory = !*show_memory );
                        }>
                    Memory
                    </button>
                    <button class = "toggle"
                        on:click=move |_| {
                            machine_state.update(|machine_state| machine_state.step(true) );
                        }>
                    Step
                    </button>
                    <button class = "toggle"
                        on:click=move |_| {
                            machine_state.emulator().running().set(false);
                            machine_state.emulator().au().set(u18::new(0));
                            machine_state.emulator().al().set(u18::new(0));
                            machine_state.emulator().p().set(u18::new(0o540));
                            machine_state.emulator().icr().set(u3::new(0));
                            machine_state.emulator().sr().set(u5::new(0));
                        }>
                        Reset
                    </button>
                    <button class = "toggle"
                        on:click=move |_| { machine_state.emulator().running().set(false); }
                        style = move || {if !machine_state.emulator().running().get() {"background-color: red; color: white"} else {""}}>
                        Stop
                    </button>
                    <button
                        class = "toggle" on:click=move |_| { machine_state.emulator().running().set(true); }
                        style = move || {if machine_state.emulator().running().get() {"background-color: green; color: white"} else {""}}>
                        Run
                    </button>
                </div>

                <div class = "subpane flex-col" style = "gap: 8px" >
                <span style = "font-weight: 900">"Stop"</span>
                    <div class = "subpane flex-col" style = "gap: 4px; padding: 0px;" >
                        <div class = "flex-row" style = "gap: 4px; justify-content: center;" >
                            <StopButton index = 5 flags = machine_state.emulator().stop() stopped_flags = machine_state.emulator().stopped() />
                        </div>
                        <div class = "flex-row" style = "gap: 4px" >
                            <StopButton index = 4 flags = machine_state.emulator().stop() stopped_flags = machine_state.emulator().stopped() />
                            <StopButton index = 3 flags = machine_state.emulator().stop() stopped_flags = machine_state.emulator().stopped() />
                            <StopButton index = 2 flags = machine_state.emulator().stop() stopped_flags = machine_state.emulator().stopped() />
                            <StopButton index = 1 flags = machine_state.emulator().stop() stopped_flags = machine_state.emulator().stopped() />
                            <StopButton index = 0 flags = machine_state.emulator().stop() stopped_flags = machine_state.emulator().stopped() />
                        </div>
                    </div>
                </div>
                <div class = "subpane flex-col" style = "gap: 8px">
                    <span style = "font-weight: 900">"Skip"</span>
                    <div class = "flex-row" style = "gap: 4px" >
                        <SkipButton index = 4 flags = machine_state.emulator().skip() />
                        <SkipButton index = 3 flags = machine_state.emulator().skip() />
                        <SkipButton index = 2 flags = machine_state.emulator().skip() />
                        <SkipButton index = 1 flags = machine_state.emulator().skip() />
                        <SkipButton index = 0 flags = machine_state.emulator().skip() />
                    </div>
                </div>
            </div>
        </div>
    }
}
