use common::emulator::StateStoreFields;
use common::io::IOStoreFields;
use common::logger;
use leptos::html::Textarea;
use leptos::prelude::*;

use crate::components::machine_state::MachineState;
use crate::components::machine_state::MachineStateStoreFields;
use reactive_stores::Store;

use common::assembler;

#[component]
pub fn AssemblyEditor(#[prop(into)] machine_state: Store<MachineState>) -> impl IntoView {
    let textarea_ref: NodeRef<Textarea> = NodeRef::new();

    view! {
        <div class = "pane" style = "display: flex;">
            <div class = "flex-col" style = "gap: 8px;">
            <div class = "flex-row" style = "justify-content: end;">
                <button class = "action-button"
                    on:click=move |_| {
                        if let Some(textarea) = textarea_ref.get() {
                            let content = textarea.value();
                            let lines = content.lines().collect();
                            let tape = assembler::assemble(&lines, true);

                            logger!("ASSEMBLED {:?}", tape);

                            machine_state.emulator().p().set(ux::u18::new(0o000540));
                            machine_state.emulator().au().set(ux::u18::new(0));
                            machine_state.emulator().al().set(ux::u18::new(0));
                            machine_state.emulator().icr().set(ux::u3::new(0));
                            machine_state.emulator().sr().set(ux::u5::new(0));

                            machine_state.emulator().io().tape().set(tape);

                            machine_state.emulator().running().set(true);
                        }
                    } >
                    Assemble
                </button>
            </div>
                <textarea class = "editor" cols = "80" spellcheck = "none" node_ref=textarea_ref>
                </textarea>
            </div>
        </div>
    }
}
