use leptos::prelude::*;

use crate::components::machine_state::MachineState;
use crate::components::machine_state::MachineStateStoreFields;
use crate::components::machine_state::TapeFile;
use common::emulator::StateStoreFields;
use common::io::IOStoreFields;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlInputElement;
use reactive_stores::Store;

use web_sys::js_sys::{ArrayBuffer, Uint8Array};

#[component]
pub fn TapeInput(#[prop(into)] machine_state: Store<MachineState>) -> impl IntoView {
    view! {
        <input type = "file" id = "tape-file-input" style = "display: none;"
            on:input=move |ev: leptos::ev::Event| {
                let target = ev.target().unwrap();
                let input: HtmlInputElement = target.unchecked_into();
                let files = input.files();

                if let Some(files) = files {
                    let length = files.length();

                    for i in 0..length {
                        let file = files.get(i).unwrap();
                        let contents_promise = wasm_bindgen_futures::JsFuture::from(file.slice().unwrap().array_buffer());

                        spawn_local(async move {
                            let contents = contents_promise.await.unwrap();


                            let file_raw_data = contents.dyn_into::<ArrayBuffer>().expect("Expected an ArrayBuffer");
                            let file_raw_data = Uint8Array::new(&file_raw_data);

                            let mut file_bytes = vec![0; file_raw_data.length() as usize];
                            file_raw_data.copy_to(file_bytes.as_mut_slice());

                            let name = file.name();

                            let tape =
                            if name.ends_with(".76") ||  name.ends_with(".76+") {
                                    let tape_str = String::from_utf8(file_bytes).unwrap();
                                    common::tape::deserialize_bioctal(&tape_str)
                                }
                                else {
                                    common::tape::deserialize_bin(&file_bytes)
                                };

                            machine_state.tape_file().set(Some(TapeFile { name, length: tape.len()}));
                            machine_state.emulator().io().tape().set(tape);
                        });
                    }
                }

            }
        > </input>
        <label for = "tape-file-input" class = "subpane flex-row" style = "gap: 4px" >
            <div class = "flex-row" style = "justify-content: space-between; align-items: center; flex-grow: 1;">
            <span style = "font-weight: 900">"Tape:"</span>
                {   move ||

                    {
                        let tape_file = machine_state.tape_file().get();
                        let io_tape_remaining = machine_state.emulator().io().tape().get().len();

                        if let Some(tape_file) = tape_file {
                            view! {
                                <div class = "flex-col">
                                    <span style = "font-weight: 500">{ tape_file.name }</span>
                                    <progress max={ tape_file.length }
                                              value= { tape_file.length - io_tape_remaining} >
                                    </progress>
                                </div>
                            }.into_any()
                        }
                        else {
                            view! { <div class = "flex-col"
                                         style = "width: 100%; font-weight: 600; color: #606060;">
                                        <span>Select Tape</span>
                                    </div> }.into_any()
                        }
                    }

                }

            </div>
        </label>
    }
}
